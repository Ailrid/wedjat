use super::util::{check_task, run_cmd};
use crate::config::{Config, MeshConfig};
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
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
    pub output: PathBuf, // {output_root}/models/{uuid}
}

pub(crate) struct ModelResult {
    pub id: i64,
    pub url: String,
    pub elapsed: Duration,
}
/// tileset.json 存在且有 root，就说明这次转换真的产出了东西
fn validate_tileset(output: &Path) -> Result<(), ConverterError> {
    let path = output.join("tileset.json");
    let raw = fs::read(&path)
        .map_err(|e| ConverterError::OtherError(format!("read {}: {e}", path.display())))?;

    let v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| ConverterError::OtherError(format!("parse tileset.json: {e}")))?;

    if v.get("root").is_none() {
        return Err(ConverterError::OtherError(format!(
            "tileset.json has no root: {}",
            path.display()
        )));
    }

    Ok(())
}

/// 从扩展名推断 mago 的 --inputType（目录输入取第一个文件）
fn input_type(input: &Path) -> Result<&'static str, ConverterError> {
    let ext = if input.is_dir() {
        fs::read_dir(input)?.flatten().find_map(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(str::to_lowercase)
        })
    } else {
        input
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_lowercase)
    };

    match ext.as_deref() {
        Some("obj") => Ok("obj"),
        Some("gltf") | Some("glb") => Ok("gltf"),
        Some("fbx") => Ok("fbx"),
        Some("3ds") => Ok("3ds"),
        Some("kml") => Ok("kml"),
        Some("shp") => Ok("shp"),
        Some("geojson") => Ok("geojson"),
        Some("gpkg") => Ok("gpkg"),
        Some("las") | Some("laz") => Ok("las"),
        Some("gml") => Ok("citygml"),
        Some(other) => Err(ConverterError::OtherError(format!(
            "unsupported model format: .{other} ({})",
            input.display()
        ))),
        None => Err(ConverterError::OtherError(format!(
            "cannot infer input type from {}",
            input.display()
        ))),
    }
}

async fn run_mago_tiler(
    input: &Path,
    output: &Path,
    epsg: i32,
    mc: &MeshConfig,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let heap = format!("-Xmx{}", mc.max_heap);
    let crs = epsg.to_string();
    let input_type = input_type(input)?;

    let mut cmd = Command::new(&mc.java_bin);
    cmd.arg(&heap)
        .arg("-Xms256m")
        .arg("-XX:MaxDirectMemorySize=512m")
        // OOM 时立刻非零退出，而不是死个线程继续跑
        .arg("-XX:+ExitOnOutOfMemoryError")
        .arg("-Djava.awt.headless=true")
        .arg("-jar")
        .arg(&mc.jar_path)
        .arg("-i")
        .arg(input)
        .arg("-o")
        .arg(output)
        .args(["--inputType", input_type])
        // 坐标系由调用方声明，不让 mago 去猜
        .args(["--crs", &crs]);

    run_cmd(cmd, "mago-3d-tiler", timeout).await.map(|_| ())
}

async fn prepare(
    output_root: &str,
    task: &Task,
    pool: &PgPool,
) -> Result<PreparedData, ConverterError> {
    let root = Path::new(output_root);
    let url = Uuid::new_v4().to_string();
    let output = root.join("models").join(&url);
    let output_s = output.to_string_lossy().to_string();

    fs::create_dir_all(root.join("models"))?;

    // 3D Tiles 是 ECEF 坐标；extra 里标记这是模型而不是点云
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO resources (name, category, source_path, url, group_name, epsg, status, extra)
         VALUES ($1, 'tiles', $2, $3, $4, 4978, 'pending', $5)
         RETURNING id",
    )
    .bind(&task.name)
    .bind(&output_s)
    .bind(&url)
    .bind(&task.group_name)
    .bind(serde_json::json!({ "type": "model" }))
    .fetch_one(pool)
    .await?;

    tracing::Span::current().record("id", id);
    tracing::Span::current().record("output", output_s.as_str());

    Ok(PreparedData { id, url, output })
}

pub(crate) async fn convert(
    task: Task,
    prepared: PreparedData,
    mc: &MeshConfig,
) -> Result<ModelResult, ConverterError> {
    let started = Instant::now();
    let timeout = Duration::from_secs(mc.timeout_secs);
    let input = Path::new(&task.source_path);

    let steps = async {
        run_mago_tiler(input, &prepared.output, task.epsg, mc, timeout).await?;
        validate_tileset(&prepared.output)
    }
    .await;

    let elapsed = started.elapsed();

    match steps {
        Ok(()) => Ok(ModelResult {
            id: prepared.id,
            url: prepared.url,
            elapsed,
        }),
        Err(e) => {
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
pub async fn mesh2tiles(task: Task, cfg: Arc<Config>, pool: PgPool) -> Result<(), ConverterError> {
    check_task(&task.source_path, &pool).await?;

    let prepared = prepare(&cfg.data.output_root, &task, &pool).await?;
    let id = prepared.id;

    let mc = &cfg.conversion.mesh;

    match convert(task, prepared, mc).await {
        Ok(result) => {
            sqlx::query(
                "UPDATE resources
                    SET status = 'ready', updated_at = now()
                  WHERE id = $1",
            )
            .bind(result.id)
            .execute(&pool)
            .await?;

            tracing::info!(
                url = %result.url,
                elapsed_ms = result.elapsed.as_millis() as u64,
                "model conversion done, marked ready"
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
