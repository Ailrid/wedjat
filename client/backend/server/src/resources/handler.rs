use super::model::{
    CreateResource, GetResource, GetResourceGroup, ListResources, ResourceResponse,
};
use super::repo;
use crate::error::AppError;
use crate::extract::Json;
use crate::pipeline::converter::{ConvertType, Task};
use crate::state::AppState;
use axum::extract::State;
use std::path::Path;

/// 根据扩展名 + gdalinfo 内容特征推断 category 和 ConvertType
async fn infer(path: &str) -> Result<(&'static str, ConvertType), AppError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| AppError::BadRequest(format!("no extension: {path}")))?;

    match ext.as_str() {
        "tif" | "tiff" => {
            if is_terrain(path).await? {
                Ok(("terrain", ConvertType::Tiff2Terrain))
            } else {
                Ok(("imagery", ConvertType::Tiff2Mbtiles))
            }
        }
        "las" | "laz" => Ok(("tiles", ConvertType::Las2Tiles)),
        "osgb" => Ok(("tiles", ConvertType::Mesh2Tiles)),
        _ => Err(AppError::BadRequest(format!(
            "unsupported file extension: {ext}"
        ))),
    }
}

/// 单波段 + Float/Int16 + 有 NoData 判定为 DEM
async fn is_terrain(path: &str) -> Result<bool, AppError> {
    let out = tokio::process::Command::new("gdalinfo")
        .args(["-json", path])
        .output()
        .await?;

    if !out.status.success() {
        return Err(AppError::BadRequest(format!(
            "gdalinfo failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }

    let info: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| AppError::BadRequest(format!("gdalinfo json parse: {e}")))?;

    let bands = info["bands"].as_array().map(|a| a.len()).unwrap_or(0);
    let band_type = info["bands"][0]["type"].as_str().unwrap_or("");
    let has_nodata = !info["bands"][0]["noDataValue"].is_null();

    Ok(bands == 1 && matches!(band_type, "Float32" | "Float64" | "Int16" | "UInt16") && has_nodata)
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateResource>,
) -> Result<Json<ResourceResponse>, AppError> {
    let (category, convert_type) = infer(&req.source_path).await?;

    // 去重：同路径已在 pending/converting 的直接返回已有记录
    if let Some(existing) = repo::latest_by_source_path(&state.pool, &req.source_path).await? {
        if existing.status == "pending" || existing.status == "converting" {
            tracing::warn!(
                source_path = %req.source_path,
                id = existing.id,
                "duplicate submit, returning existing"
            );
            return Ok(Json(existing.into()));
        }
    }

    let extra = serde_json::json!({});

    let res = repo::insert_pending(
        &state.pool,
        &req.name,
        category,
        &req.source_path,
        req.group_name.as_deref(),
        req.epsg,
        &extra,
    )
    .await?;

    let task = Task {
        name: res.name.clone(),
        source_path: res.source_path.clone(),
        group_name: res.group_name.clone(),
        epsg: res.epsg,
        convert_type,
    };
    if let Err(e) = state.converter.convert(task) {
        tracing::warn!(error = %e, id = res.id, "queue full, task stays pending");
    }

    tracing::info!(
        id = res.id,
        category,
        convert_type = ?convert_type,
        epsg = req.epsg,
        "resource submitted"
    );

    Ok(Json(res.into()))
}

pub async fn get(
    State(state): State<AppState>,
    Json(req): Json<GetResource>,
) -> Result<Json<ResourceResponse>, AppError> {
    repo::get(&state.pool, req.id)
        .await?
        .map(ResourceResponse::from)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("resource {} not found", req.id)))
}

pub async fn get_group(
    State(state): State<AppState>,
    Json(req): Json<GetResourceGroup>,
) -> Result<Json<Vec<ResourceResponse>>, AppError> {
    let rows = repo::list_by_group(&state.pool, &req.group_name).await?;
    Ok(Json(rows.into_iter().map(ResourceResponse::from).collect()))
}

pub async fn list(
    State(state): State<AppState>,
    Json(req): Json<ListResources>,
) -> Result<Json<Vec<ResourceResponse>>, AppError> {
    let rows = repo::list(&state.pool, req.category.as_deref(), req.status.as_deref()).await?;
    Ok(Json(rows.into_iter().map(ResourceResponse::from).collect()))
}
