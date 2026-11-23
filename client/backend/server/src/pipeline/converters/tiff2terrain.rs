use super::util::{check_task, run_cmd};
use crate::config::{Config, TerrainConfig};
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
use serde::Deserialize;
use sqlx::PgPool;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use uuid::Uuid;

pub(crate) struct PreparedData {
    pub id: i64,
    pub url: String,
    pub output: PathBuf, // {output_root}/terrain/{uuid}
    pub temp: PathBuf,   // {output_root}/temp/{uuid}
    pub dem: PathBuf,    // {temp}/dem.tif
}

/// 转换产出的全部事实
pub(crate) struct TerrainResult {
    pub id: i64,
    pub url: String,
    pub min_zoom: i32,
    pub max_zoom: i32,
    pub bounds: [f64; 4], // [west, south, east, north]
    pub elapsed: Duration,
}

#[derive(Debug, Deserialize)]
struct LayerJson {
    bounds: [f64; 4],
    #[serde(default)]
    available: Vec<serde_json::Value>,
}

fn read_layer_json(output: &Path) -> Result<LayerJson, ConverterError> {
    let path = output.join("layer.json");
    let raw = fs::read(&path)
        .map_err(|e| ConverterError::OtherError(format!("read {}: {e}", path.display())))?;

    serde_json::from_slice(&raw)
        .map_err(|e| ConverterError::OtherError(format!("parse layer.json: {e}")))
}

/// 产物目录里实际存在的 z 层，顺带检查断档
fn scan_zoom_range(dir: &Path) -> Result<(i32, i32), ConverterError> {
    let mut zooms: Vec<i32> = fs::read_dir(dir)
        .map_err(|e| ConverterError::OtherError(format!("read {}: {e}", dir.display())))?
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse().ok()))
        .collect();
    zooms.sort_unstable();

    let contiguous = zooms.windows(2).all(|w| w[1] == w[0] + 1);

    match (zooms.first(), zooms.last()) {
        (Some(&min), Some(&max)) if contiguous => Ok((min, max)),
        (None, _) => Err(ConverterError::OtherError(format!(
            "no zoom dir in {}",
            dir.display()
        ))),
        _ => Err(ConverterError::OtherError(format!(
            "zoom dirs not contiguous: {zooms:?}"
        ))),
    }
}

/// 补 minzoom/maxzoom，换掉 mago 的占位符，删掉非标准字段
fn finalize_layer_json(
    output: &Path,
    name: &str,
    attribution: &str,
    min_zoom: i32,
    max_zoom: i32,
) -> Result<(), ConverterError> {
    let path = output.join("layer.json");
    let raw = fs::read(&path)?;

    let mut v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| ConverterError::OtherError(format!("parse layer.json: {e}")))?;

    let obj = v
        .as_object_mut()
        .ok_or_else(|| ConverterError::OtherError("layer.json is not an object".into()))?;

    // Cesium 不读这两个字段，但 DB / 其他客户端要看
    obj.insert("minzoom".into(), min_zoom.into());
    obj.insert("maxzoom".into(), max_zoom.into());

    // mago 写的是 "insert name here" 之类的占位符
    obj.insert("name".into(), name.into());
    obj.insert("description".into(), "".into());
    obj.insert("attribution".into(), attribution.into());

    // mago 自留字段，不是 TileJSON 规范
    obj.remove("template");
    obj.remove("legend");

    let out = serde_json::to_vec(&v)
        .map_err(|e| ConverterError::OtherError(format!("serialize layer.json: {e}")))?;

    fs::write(&path, out)?;
    Ok(())
}

/// 源 DEM -> EPSG:4326（mago 只认地理坐标）
async fn prepare_dem(
    input: &Path,
    dem: &Path,
    epsg: i32,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let mut cmd = Command::new("gdalwarp");
    cmd.args([
        "-s_srs",
        &format!("EPSG:{epsg}"),
        "-t_srs",
        "EPSG:4326",
        "-r",
        "bilinear",
        "-overwrite",
        "-multi",
        "-wo",
        "NUM_THREADS=ALL_CPUS",
        "-co",
        "TILED=YES",
        "-co",
        "COMPRESS=DEFLATE",
        "-co",
        "BIGTIFF=IF_SAFER",
    ])
    .arg(input)
    .arg(dem);

    run_cmd(cmd, "gdalwarp", timeout).await.map(|_| ())
}

/// 调 mago 生成 quantized-mesh 金字塔
async fn run_mago(
    dem: &Path,
    output: &Path,
    temp: &Path,
    tc: &TerrainConfig,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let heap = format!("-Xmx{}", tc.max_heap);
    let max_depth = tc.max_depth.to_string();
    let nodata = tc.nodata_value.to_string();
    let raster_max = tc.raster_max_size.to_string();
    let log_path = temp.join("mago.log");

    let mut cmd = Command::new(&tc.java_bin);
    cmd.arg(&heap)
        .arg("-Xms256m")
        .arg("-XX:MaxDirectMemorySize=512m")
        // OOM 时立刻非零退出，而不是死个线程继续跑
        .arg("-XX:+ExitOnOutOfMemoryError")
        .arg("-Djava.awt.headless=true")
        .arg("-jar")
        .arg(&tc.jar_path)
        .arg("-i")
        .arg(dem)
        .arg("-o")
        .arg(output)
        .arg("-t")
        .arg(temp)
        .args(["-max", &max_depth])
        .args(["-nv", &nodata])
        .args(["-g", tc.geoid.as_str()])
        .args(["-mr", &raster_max])
        .arg("-l")
        .arg(&log_path)
        .arg("-q");

    // 不要加 -j：它会短路去重建 layer.json 而不生成瓦片
    run_cmd(cmd, "mago-3d-terrainer", timeout).await.map(|_| ())
}

async fn prepare(
    output_root: &str,
    task: &Task,
    pool: &PgPool,
) -> Result<PreparedData, ConverterError> {
    let root = Path::new(output_root);
    let url = Uuid::new_v4().to_string();
    let output = root.join("terrain").join(&url);
    let temp = root.join("temp").join(&url);
    let dem = temp.join("dem.tif");
    let output_s = output.to_string_lossy().to_string();

    fs::create_dir_all(&temp)?;

    // 产物是 4326 地理切片
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO resources (name, category, source_path, url, group_name, epsg, status)
         VALUES ($1, 'terrain', $2, $3, $4, 4326, 'pending')
         RETURNING id",
    )
    .bind(&task.name)
    .bind(&output_s)
    .bind(&url)
    .bind(&task.group_name)
    .fetch_one(pool)
    .await?;

    tracing::Span::current().record("id", id);
    tracing::Span::current().record("output", output_s.as_str());

    Ok(PreparedData {
        id,
        url,
        output,
        temp,
        dem,
    })
}

/// 重投影 -> mago -> 定层级 -> 回写 layer.json，只返回事实
async fn run_steps(
    task: &Task,
    prepared: &PreparedData,
    tc: &TerrainConfig,
    timeout: Duration,
) -> Result<(i32, i32, [f64; 4]), ConverterError> {
    let input = Path::new(&task.source_path);

    prepare_dem(input, &prepared.dem, task.epsg, timeout).await?;
    run_mago(&prepared.dem, &prepared.output, &prepared.temp, tc, timeout).await?;

    let layer = read_layer_json(&prepared.output)?;
    let (min_zoom, max_zoom) = scan_zoom_range(&prepared.output)?;

    // available 说有几层就必须真有那么多层，否则是中途挂过
    if !layer.available.is_empty() {
        let claimed = layer.available.len() as i32 - 1;
        if claimed != max_zoom {
            return Err(ConverterError::OtherError(format!(
                "layer.json claims z0..z{claimed}, tiles only reach z{max_zoom}"
            )));
        }
    }

    finalize_layer_json(
        &prepared.output,
        &task.name,
        &tc.attribution,
        min_zoom,
        max_zoom,
    )?;

    Ok((min_zoom, max_zoom, layer.bounds))
}

pub(crate) async fn convert(
    task: Task,
    prepared: PreparedData,
    tc: &TerrainConfig,
) -> Result<TerrainResult, ConverterError> {
    let started = Instant::now();
    let timeout = Duration::from_secs(tc.timeout_secs);

    let steps = run_steps(&task, &prepared, tc, timeout).await;
    let elapsed = started.elapsed();

    match steps {
        Ok((min_zoom, max_zoom, bounds)) => {
            // 清理失败不影响转换结果
            let _ = fs::remove_dir_all(&prepared.temp);

            Ok(TerrainResult {
                id: prepared.id,
                url: prepared.url,
                min_zoom,
                max_zoom,
                bounds,
                elapsed,
            })
        }
        Err(e) => {
            // 失败保留 temp：mago.log 是唯一能看出原因的东西
            let _ = fs::remove_dir_all(&prepared.output);
            Err(e)
        }
    }
}

#[tracing::instrument(
    skip(cfg, pool),
    err,
    fields(
        input = %task.source_path,
        name = %task.name,
        convert_type = ?task.convert_type,
        id = tracing::field::Empty,
        output = tracing::field::Empty,
    )
)]
pub async fn tiff2terrain(
    task: Task,
    cfg: Arc<Config>,
    pool: PgPool,
) -> Result<(), ConverterError> {
    check_task(&task.source_path, &pool).await?;

    let prepared = prepare(&cfg.data.output_root, &task, &pool).await?;
    let id = prepared.id;

    let tc = &cfg.conversion.terrain;

    match convert(task, prepared, tc).await {
        Ok(result) => {
            sqlx::query(
                "UPDATE resources
                    SET status     = 'ready',
                        min_zoom   = $2,
                        max_zoom   = $3,
                        min_lon    = $4,
                        min_lat    = $5,
                        max_lon    = $6,
                        max_lat    = $7,
                        updated_at = now()
                  WHERE id = $1",
            )
            .bind(result.id)
            .bind(result.min_zoom)
            .bind(result.max_zoom)
            .bind(result.bounds[0])
            .bind(result.bounds[1])
            .bind(result.bounds[2])
            .bind(result.bounds[3])
            .execute(&pool)
            .await?;

            tracing::info!(
                url = %result.url,
                min_zoom = result.min_zoom,
                max_zoom = result.max_zoom,
                bbox = ?result.bounds,
                elapsed_ms = result.elapsed.as_millis() as u64,
                "terrain conversion done, marked ready"
            );
            Ok(())
        }
        Err(e) => {
            sqlx::query(
                "UPDATE resources
                    SET status = 'failed',
                        error_message = $1,
                        updated_at = now()
                  WHERE id = $2",
            )
            .bind(e.to_string())
            .bind(id)
            .execute(&pool)
            .await?;

            Err(e)
        }
    }
}
