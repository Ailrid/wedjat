use super::util::run_cmd;
use crate::config::{Config, PointCloudConfig};
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
use serde::Deserialize;
use sqlx::PgPool;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;

pub(crate) struct PreparedData {
    pub url: String,
    pub output: PathBuf, // {output_root}/tiles/{uuid}
}

pub(crate) struct TilesResult {
    pub url: String,
    pub bounds: Option<[f64; 4]>, // [west, south, east, north]，度
    pub elapsed: Duration,
}

/// tileset.json 只取根包围体，用来填 DB 的 bbox
#[derive(Debug, Deserialize)]
struct Tileset {
    root: Root,
}

#[derive(Debug, Deserialize)]
struct Root {
    #[serde(rename = "boundingVolume")]
    bounding_volume: BoundingVolume,
}

#[derive(Debug, Deserialize)]
struct BoundingVolume {
    #[serde(default)]
    region: Option<[f64; 6]>,
}

fn read_tileset(output: &Path) -> Result<Tileset, ConverterError> {
    let path = output.join("tileset.json");
    let raw = fs::read(&path)
        .map_err(|e| ConverterError::OtherError(format!("read {}: {e}", path.display())))?;

    serde_json::from_slice(&raw)
        .map_err(|e| ConverterError::OtherError(format!("parse tileset.json: {e}")))
}

async fn run_gotiler(
    input: &Path,
    output: &Path,
    epsg: i32,
    pc: &PointCloudConfig,
    timeout: Duration,
) -> Result<(), ConverterError> {
    let points = pc.points_per_tile.to_string();

    let mut cmd = Command::new(&pc.bin_path);
    cmd.arg("pcloud")
        .arg("-o")
        .arg(output)
        .args(["-c", &format!("EPSG:{epsg}")])
        .args(["--points-per-tile", &points]);

    if !pc.colorize.is_empty() {
        cmd.args(["--colorize", pc.colorize.as_str()]);
    }

    // 输出目录形式，Cesium 直接读 tileset.json
    cmd.arg(input);
    run_cmd(cmd, "gotiler", timeout).await.map(|_| ())
}

/// 行已由 handler 插入。这里只做两件事：占住这一行（pending -> converting，写入 url/output_path），准备目录
async fn prepare(
    output_root: &str,
    task: &Task,
    pool: &PgPool,
) -> Result<PreparedData, ConverterError> {
    let id = task.id.to_string();
    let output = Path::new(output_root).join("tiles").join(&id);
    let output_s = output.to_string_lossy().to_string();
    let url = format!("/api/data/tiles/{id}/tileset.json");

    fs::create_dir_all(&output)?;

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
        // 行被删了，或者已经被另一个任务接管了，别白跑
        let _ = fs::remove_dir_all(&output);
        return Err(ConverterError::OtherError(format!(
            "resource {} is not pending anymore",
            task.id
        )));
    }

    tracing::Span::current().record("output", output_s.as_str());

    Ok(PreparedData { url, output })
}

/// 拿走所有权，跑完返回 TilesResult
pub(crate) async fn convert(
    task: Task,
    prepared: PreparedData,
    pc: &PointCloudConfig,
) -> Result<TilesResult, ConverterError> {
    let started = Instant::now();
    let timeout = Duration::from_secs(pc.timeout_secs);
    let input = Path::new(&task.source_path);

    let steps = async {
        run_gotiler(input, &prepared.output, task.epsg, pc, timeout).await?;
        read_tileset(&prepared.output)
    }
    .await;

    let elapsed = started.elapsed();

    match steps {
        Ok(tileset) => {
            let bounds = tileset.root.bounding_volume.region.map(|r| {
                [
                    r[0].to_degrees(),
                    r[1].to_degrees(),
                    r[2].to_degrees(),
                    r[3].to_degrees(),
                ]
            });

            Ok(TilesResult {
                url: prepared.url,
                bounds,
                elapsed,
            })
        }
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
        id = %task.id,
        output = tracing::field::Empty,
    )
)]
pub async fn las2tiles(task: Task, cfg: Arc<Config>, pool: PgPool) -> Result<(), ConverterError> {
    let id = task.id;

    let prepared = prepare(&cfg.data.output_root, &task, &pool).await?;

    let pc = &cfg.conversion.pointcloud;

    match convert(task, prepared, pc).await {
        Ok(result) => {
            let (w, s, e, n) = match result.bounds {
                Some(b) => (Some(b[0]), Some(b[1]), Some(b[2]), Some(b[3])),
                None => (None, None, None, None),
            };

            // 3D Tiles 没有 zoom 概念，min_zoom/max_zoom 保持 NULL
            sqlx::query(
                "UPDATE resources
                    SET status     = 'ready',
                        min_lon    = $2,
                        min_lat    = $3,
                        max_lon    = $4,
                        max_lat    = $5,
                        updated_at = now()
                  WHERE id = $1",
            )
            .bind(id)
            .bind(w)
            .bind(s)
            .bind(e)
            .bind(n)
            .execute(&pool)
            .await?;

            tracing::info!(
                url = %result.url,
                bbox = ?result.bounds,
                elapsed_ms = result.elapsed.as_millis() as u64,
                "point cloud conversion done, marked ready"
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
