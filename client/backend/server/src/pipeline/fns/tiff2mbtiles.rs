use crate::config::Config;
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
use sqlx::PgPool;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use uuid::Uuid;

use std::path::PathBuf;

async fn run_gdal(input: &str, output: &str, epsg: i32) -> Result<(), ConverterError> {
    // 中间文件：重投影后的 3857 GeoTIFF
    let warped: PathBuf = PathBuf::from(format!("{}.warped.tif", output));

    // 1. 显式重投影：s_srs = 调用方声明的源坐标系，t_srs = Web 墨卡托
    let warp = Command::new("gdalwarp")
        .args([
            "-s_srs",
            &format!("EPSG:{}", epsg),
            "-t_srs",
            "EPSG:3857",
            "-r",
            "bilinear",
            "-overwrite",
            input,
            warped.to_str().unwrap_or_default(),
        ])
        .output()
        .await?;

    if !warp.status.success() {
        let _ = tokio::fs::remove_file(&warped).await;
        return Err(ConverterError::CommandFailed {
            cmd: "gdalwarp".into(),
            stderr: String::from_utf8_lossy(&warp.stderr).to_string(),
        });
    }

    // 2. gdal_translate：输入现在是确定的 3857，直接切片
    let translate = Command::new("gdal_translate")
        .args([
            warped.to_str().unwrap_or_default(),
            output,
            "-of",
            "MBTiles",
            "-co",
            "TILE_FORMAT=PNG",
        ])
        .output()
        .await?;

    // 清理中间文件，无论成功失败
    let _ = tokio::fs::remove_file(&warped).await;

    if !translate.status.success() {
        return Err(ConverterError::CommandFailed {
            cmd: "gdal_translate".into(),
            stderr: String::from_utf8_lossy(&translate.stderr).to_string(),
        });
    }

    // 3. gdaladdo：生成金字塔层级
    //    2/4/8/16/32/64 是相对基础层级的降采样因子。
    //    例如基础层是 Z18，则依次生成 Z17、Z16、Z15、Z14、Z13、Z12。
    let addo = Command::new("gdaladdo")
        .args([
            "-r", "average", output, "2", "4", "8", "16", "32", "64", "128", "256",
        ])
        .output()
        .await?;

    if !addo.status.success() {
        return Err(ConverterError::CommandFailed {
            cmd: "gdaladdo".into(),
            stderr: String::from_utf8_lossy(&addo.stderr).to_string(),
        });
    }

    Ok(())
}
pub async fn tiff2mbtiles(
    task: Task,
    cfg: Arc<Config>,
    pool: PgPool,
) -> Result<(), ConverterError> {
    let input = &task.source_path;

    // 按路径查原始记录，failed / converting 都跳过
    let (status,): (String,) = sqlx::query_as(
        "SELECT status FROM resources WHERE source_path = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(input)
    .fetch_one(&pool)
    .await?;

    if status == "failed" || status == "converting" {
        tracing::warn!(
            input = %input,
            status = %status,
            "skip conversion: source is failed or already converting"
        );
        return Ok(());
    }

    // slug 作为产物文件的名称，同时作为 Martin 的 URL 后缀
    let url = Uuid::new_v4().to_string();
    let output = format!("{}/{}.mbtiles", cfg.data.output_root, url);

    if let Some(parent) = Path::new(&output).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 产物记录：source_path = 产物路径，url = Martin 访问前缀，group_name 透传
    let new_id: i64 = sqlx::query_scalar(
        "INSERT INTO resources (name, category, source_path, url, group_name,epsg, status)
         VALUES ($1, 'imagery', $2, $3, $4, $5,'pending')
         RETURNING id",
    )
    .bind(&task.name)
    .bind(&output)
    .bind(&url)
    .bind(&task.group_name)
    .bind(task.epsg)
    .fetch_one(&pool)
    .await?;

    tracing::info!(new_id, "product record inserted, starting conversion");

    match run_gdal(input, &output, task.epsg).await {
        Ok(()) => {
            sqlx::query(
                "UPDATE resources
                 SET status = 'ready',
                     min_zoom = 12,
                     max_zoom = 18,
                     updated_at = now()
                 WHERE id = $1",
            )
            .bind(new_id)
            .execute(&pool)
            .await?;
            tracing::info!(new_id, url = %url, "conversion done, marked ready");
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
            .bind(new_id)
            .execute(&pool)
            .await?;
            tracing::error!(new_id, error = %e, "conversion failed");
            Err(e)
        }
    }
}
