use super::util::run_cmd;
use crate::config::{Config, MbtilesConfig};
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
use sqlx::PgPool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// 只读打开产物 mbtiles
async fn open_mbtiles(path: &Path) -> Result<sqlx::SqlitePool, ConverterError> {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| ConverterError::OtherError(format!("open {}: {e}", path.display())))
}

/// 直接问 SQLite：tiles 表里现有的层级范围
async fn read_zoom_range(path: &Path) -> Result<(i32, i32), ConverterError> {
    let pool = open_mbtiles(path).await?;

    let row: (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT MIN(zoom_level), MAX(zoom_level) FROM tiles")
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                ConverterError::OtherError(format!("read zoom_level from {}: {e}", path.display()))
            })?;

    let _ = pool.close().await;

    match row {
        (Some(min), Some(max)) => Ok((min as i32, max as i32)),
        _ => Err(ConverterError::OtherError(format!(
            "tiles table is empty in {}",
            path.display()
        ))),
    }
}

/// 基础层 -> 需要补的降采样因子（2 的幂）
fn overview_factors(base_zoom: i32, min_zoom: i32) -> Vec<String> {
    let levels = (base_zoom - min_zoom).clamp(0, base_zoom.max(0));
    (1..=levels).map(|i| (1u64 << i).to_string()).collect()
}

fn thread_arg(threads: usize) -> String {
    if threads == 0 {
        "ALL_CPUS".to_string()
    } else {
        threads.to_string()
    }
}

async fn warp_to_mercator(
    input: &Path,
    warped: &Path,
    epsg: i32,
    threads: usize,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let mut cmd = Command::new("gdalwarp");
    cmd.args([
        "-s_srs",
        &format!("EPSG:{epsg}"),
        "-t_srs",
        "EPSG:3857",
        "-r",
        "bilinear",
        "-overwrite",
        "-multi",
        "-wo",
        &format!("NUM_THREADS={}", thread_arg(threads)),
        "-co",
        "TILED=YES",
        "-co",
        "COMPRESS=DEFLATE",
        "-co",
        "BIGTIFF=IF_SAFER",
    ])
    .arg(input)
    .arg(warped);

    run_cmd(cmd, "gdalwarp", timeout).await.map(|_| ())
}

async fn translate_to_mbtiles(
    warped: &Path,
    output: &Path,
    name: &str,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let mut cmd = Command::new("gdal_translate");
    cmd.args([
        "-of",
        "MBTiles",
        "-co",
        "TILE_FORMAT=PNG",
        "-co",
        &format!("NAME={name}"),
    ])
    .arg(warped)
    .arg(output);

    run_cmd(cmd, "gdal_translate", timeout).await.map(|_| ())
}

async fn build_overviews(
    output: &Path,
    factors: &[String],
    timeout: Duration,
) -> Result<(), ConverterError> {
    let mut cmd = Command::new("gdaladdo");
    cmd.args(["-r", "average"]).arg(output).args(factors);

    run_cmd(cmd, "gdaladdo", timeout).await.map(|_| ())
}

/// 转换前就绪的东西：路径 + url（= Martin 图层名）
pub(crate) struct PreparedData {
    pub url: String,
    pub output: PathBuf,
    pub temp: PathBuf,
}

/// 转换产出的全部事实
pub(crate) struct ConvertResult {
    pub url: String,
    pub min_zoom: i32,
    pub max_zoom: i32,
    pub overviews: Vec<String>,
    pub elapsed: Duration,
}

/// 行已由 handler 插入。这里占住这一行并登记产物路径，然后准备目录
async fn prepare(
    output_root: &str,
    task: &Task,
    pool: &PgPool,
) -> Result<PreparedData, ConverterError> {
    // {output_root}/mbtiles 放产物，{output_root}/temp 放中间文件
    let root = Path::new(output_root);
    let mbtiles_dir = root.join("mbtiles");
    let temp_dir = root.join("temp");

    std::fs::create_dir_all(&mbtiles_dir)?;
    std::fs::create_dir_all(&temp_dir)?;

    // url 同时作为产物文件名和 Martin 的访问前缀
    let id = task.id.to_string();
    let output = mbtiles_dir.join(format!("{id}.mbtiles"));
    let temp = temp_dir.join(format!("{id}.warped.tif"));
    let output_s = output.to_string_lossy().to_string();
    let url = id;

    let affected = sqlx::query(
        "UPDATE resources
            SET status      = 'converting',
                url         = $2,
                output_path = $3,
                updated_at  = now()
          WHERE id = $1
            AND status = 'pending'",
    )
    .bind(task.id)
    .bind(&url)
    .bind(&output_s)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ConverterError::OtherError(format!(
            "resource {} is not pending anymore",
            task.id
        )));
    }

    tracing::Span::current().record("output", output_s.as_str());

    Ok(PreparedData { url, output, temp })
}

/// 四步跑完，只返回事实
async fn run_steps(
    task: &Task,
    prepared: &PreparedData,
    mb: &MbtilesConfig,
    timeout: Duration,
) -> Result<(i32, i32, Vec<String>), ConverterError> {
    let input = Path::new(&task.source_path);

    warp_to_mercator(input, &prepared.temp, task.epsg, mb.threads, timeout).await?;
    translate_to_mbtiles(&prepared.temp, &prepared.output, &task.name, timeout).await?;

    // 此时 tiles 表里只有基础层一层
    let (_, base_zoom) = read_zoom_range(&prepared.output).await?;

    let factors = overview_factors(base_zoom, mb.min_zoom as i32);
    if factors.is_empty() {
        return Ok((base_zoom, base_zoom, factors));
    }

    build_overviews(&prepared.output, &factors, timeout).await?;

    // 金字塔建完再问一次 —— 这才是真正写进去的范围
    let (min_zoom, max_zoom) = read_zoom_range(&prepared.output).await?;

    Ok((min_zoom, max_zoom, factors))
}

pub(crate) async fn convert(
    task: Task,
    prepared: PreparedData,
    mb: &MbtilesConfig,
) -> Result<ConvertResult, ConverterError> {
    let started = Instant::now();
    let timeout = Duration::from_secs(mb.timeout_secs);

    let steps = run_steps(&task, &prepared, mb, timeout).await;
    let elapsed = started.elapsed();

    // 中间文件无论如何都删，删不掉也不该覆盖真正的错误
    let _ = std::fs::remove_file(&prepared.temp);

    let (min_zoom, max_zoom, overviews) = match steps {
        Ok(v) => v,
        Err(e) => {
            // 失败时半成品也要删，否则 Martin 会扫到残缺的库
            let _ = std::fs::remove_file(&prepared.output);
            return Err(e);
        }
    };

    Ok(ConvertResult {
        url: prepared.url,
        min_zoom,
        max_zoom,
        overviews,
        elapsed,
    })
}

#[tracing::instrument(
    skip(cfg, pool),
    err,
    fields(
        input = %task.source_path,
        name = %task.name,
        convert_type = ?task.convert_type,
        id = %task.id,
        output = tracing::field::Empty,
    )
)]
pub async fn tiff2mbtiles(
    task: Task,
    cfg: Arc<Config>,
    pool: PgPool,
) -> Result<(), ConverterError> {
    let id = task.id;

    let prepared = prepare(&cfg.data.output_root, &task, &pool).await?;

    match convert(task, prepared, &cfg.conversion.mbtiles).await {
        Ok(result) => {
            sqlx::query(
                "UPDATE resources
                    SET status = 'ready',
                        min_zoom = $2,
                        max_zoom = $3,
                        updated_at = now()
                  WHERE id = $1",
            )
            .bind(id)
            .bind(result.min_zoom)
            .bind(result.max_zoom)
            .execute(&pool)
            .await?;

            tracing::info!(
                url = %result.url,
                min_zoom = result.min_zoom,
                max_zoom = result.max_zoom,
                overviews = result.overviews.len(),
                elapsed_ms = result.elapsed.as_millis() as u64,
                "conversion done, marked ready"
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
