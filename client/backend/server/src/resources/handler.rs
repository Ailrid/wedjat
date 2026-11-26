use super::model::{
    CreateBase, CreateImageryRequest, CreateResourceResponse, CreateTerrainRequest,
    CreateTilesRequest, DeleteResourcesRequest, DeleteResourcesResponse, ResourceExtra,
    ResourceFiltered, ResourcesListRequest, ResourcesListResponse, SortStyle, TilesConfig,
    TilingScheme, XyzTag,
};
use super::repo;
use crate::error::AppError;
use crate::extract::Json;
use crate::pipeline::converter::{ConvertType, Task};
use crate::state::AppState;
use axum::extract::State;
use std::collections::HashSet;
use std::path::Path;

const TIFF_EXT: &[&str] = &["tif", "tiff"];
const LAS_EXT: &[&str] = &["las", "laz"];

const MESH_EXT: &[&str] = &[
    "osgb", "obj", "gltf", "glb", "fbx", "3ds", "kml", "shp", "geojson", "gpkg", "gml",
];

// ---------------------------------------------------------------- 校验

fn validate_file(source_path: &str, allowed: &[&str], what: &str) -> Result<(), AppError> {
    let path = Path::new(source_path);

    if !path.is_file() {
        return Err(AppError::BadRequest(format!(
            "source is not a readable file: {source_path}"
        )));
    }

    let ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| allowed.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false);

    if !ok {
        return Err(AppError::BadRequest(format!(
            "{what} requires {allowed:?}, got: {source_path}"
        )));
    }

    Ok(())
}

/// 三维模型：目录（osgb）或单文件
fn validate_mesh_source(source_path: &str) -> Result<(), AppError> {
    let path = Path::new(source_path);

    if path.is_dir() {
        let empty = std::fs::read_dir(path)
            .map_err(|e| AppError::BadRequest(format!("cannot read {source_path}: {e}")))?
            .next()
            .is_none();

        if empty {
            return Err(AppError::BadRequest(format!(
                "source directory is empty: {source_path}"
            )));
        }

        return Ok(());
    }

    validate_file(source_path, MESH_EXT, "model")
}

// ---------------------------------------------------------------- 共用入口

/// 三条创建路径的唯一出口：去重 → 插行 → 入队 → 失败回写
async fn submit(
    state: &AppState,
    base: CreateBase,
    category: &'static str,
    convert_type: ConvertType,
    extra: ResourceExtra,
) -> Result<Json<CreateResourceResponse>, AppError> {
    if let Some(existing) = repo::active_by_source_path(&state.pool, &base.source_path).await? {
        return Err(AppError::ConflictError(format!(
            "resource {} is already {} for {}",
            existing.id, existing.status, base.source_path
        )));
    }

    let row = repo::insert(
        &state.pool,
        &base.name,
        category,
        &base.source_path,
        base.group_name.as_deref(),
        base.epsg,
        &extra,
    )
    .await?;

    tracing::info!(id = row.id, category, source = %base.source_path, "resource submitted");

    let task = Task {
        id: row.id,
        name: base.name,
        source_path: base.source_path,
        epsg: base.epsg,
        convert_type,
    };

    if let Err(e) = state.converter.convert(task) {
        repo::mark_failed(&state.pool, row.id, &e.to_string()).await?;
        return Err(e.into());
    }

    Ok(Json(CreateResourceResponse {
        code: 200,
        id: row.id,
    }))
}

// ---------------------------------------------------------------- 创建

pub async fn create_imagery(
    State(state): State<AppState>,
    Json(req): Json<CreateImageryRequest>,
) -> Result<Json<CreateResourceResponse>, AppError> {
    let CreateImageryRequest { base, mut extra } = req;

    validate_file(&base.source_path, TIFF_EXT, "imagery")?;

    // 后端说了算的字段，覆盖前端传的
    extra.kind = XyzTag::Xyz;
    extra.subdomains.clear();
    extra.tiling_scheme = TilingScheme::WebMercator;
    extra.minimum_level = 0;
    extra.maximum_level = 18;

    let extra = ResourceExtra {
        imagery: Some(extra),
        ..Default::default()
    };

    submit(&state, base, "imagery", ConvertType::Tiff2Mbtiles, extra).await
}

pub async fn create_terrain(
    State(state): State<AppState>,
    Json(req): Json<CreateTerrainRequest>,
) -> Result<Json<CreateResourceResponse>, AppError> {
    let CreateTerrainRequest { base, mut extra } = req;

    validate_file(&base.source_path, TIFF_EXT, "terrain")?;

    // mago 的产物没有 water mask
    extra.request_vertex_normals = true;
    extra.request_water_mask = false;

    let extra = ResourceExtra {
        terrain: Some(extra),
        ..Default::default()
    };

    submit(&state, base, "terrain", ConvertType::Tiff2Terrain, extra).await
}

pub async fn create_tiles(
    State(state): State<AppState>,
    Json(req): Json<CreateTilesRequest>,
) -> Result<Json<CreateResourceResponse>, AppError> {
    let CreateTilesRequest { base, extra } = req;

    match extra {
        TilesConfig::Las(pc) => {
            validate_file(&base.source_path, LAS_EXT, "pointcloud")?;

            let extra = ResourceExtra {
                tiles: Some(TilesConfig::Las(pc)),
                ..Default::default()
            };

            submit(&state, base, "tiles", ConvertType::Las2Tiles, extra).await
        }
        TilesConfig::Mesh(mesh) => {
            validate_mesh_source(&base.source_path)?;

            let extra = ResourceExtra {
                tiles: Some(TilesConfig::Mesh(mesh)),
                ..Default::default()
            };

            submit(&state, base, "tiles", ConvertType::Mesh2Tiles, extra).await
        }
    }
}

// ---------------------------------------------------------------- 列表

pub async fn list(
    State(state): State<AppState>,
    Json(req): Json<ResourcesListRequest>,
) -> Result<Json<ResourcesListResponse>, AppError> {
    // 排序片段全部是字面量，用户输入只决定走哪个分支
    let order_by = match req.sort_style {
        SortStyle::NameAsc => "name ASC, id DESC",
        SortStyle::NameDesc => "name DESC, id DESC",
        SortStyle::TimeAsc => "created_at ASC, id ASC",
        SortStyle::TimeDesc => "created_at DESC, id DESC",
    };

    let start = req.start.max(0);
    let limit = req.limit;

    let (total, rows) =
        repo::list_ready(&state.pool, req.category.as_str(), order_by, start, limit).await?;

    Ok(Json(ResourcesListResponse {
        code: 200,
        resources: rows.into_iter().map(ResourceFiltered::from).collect(),
        total: total,
    }))
}

// ---------------------------------------------------------------- 删除
/// 产物可能是目录（tiles/terrain/mesh）也可能是单文件（mbtiles）
fn remove_output(path: &str) {
    let p = Path::new(path);

    let result = if p.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    };

    match result {
        Ok(()) => tracing::info!(path, "output removed"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path, "output already gone");
        }
        Err(e) => tracing::warn!(path, error = %e, "failed to remove output"),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Json(req): Json<DeleteResourcesRequest>,
) -> Result<Json<DeleteResourcesResponse>, AppError> {
    if req.ids.is_empty() {
        return Err(AppError::BadRequest("ids is empty".into()));
    }

    // 去重
    let mut wanted = req.ids;
    wanted.sort_unstable();
    wanted.dedup();

    let deleted = repo::delete_finished(&state.pool, &wanted).await?;
    let deleted_ids: HashSet<i64> = deleted.iter().map(|(id, _)| *id).collect();

    // 删文件
    for (_, output) in &deleted {
        if let Some(path) = output {
            remove_output(path);
        }
    }

    // 没删成的要么不存在，要么正在转换
    let failed: Vec<i64> = wanted
        .into_iter()
        .filter(|id| !deleted_ids.contains(id))
        .collect();

    tracing::info!(deleted = ?deleted_ids, failed = ?failed, "resources deleted");

    Ok(Json(DeleteResourcesResponse {
        code: if failed.is_empty() { 200 } else { 201 },
        failed,
    }))
}
