use serde::{Deserialize, Serialize};
use sqlx::types::chrono;

// ---------------- 影像----------------
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XyzTag {
    #[default]
    Xyz, // → "xyz"
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum TilingScheme {
    #[default]
    WebMercator,
    Geographic, // 只是为了让反序列化不失败，永远会被 normalize 覆盖
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageryConfig {
    #[serde(rename = "type")]
    pub kind: XyzTag,
    pub credit: String,
    pub subdomains: String,
    pub minimum_level: u32,
    pub maximum_level: u32,
    pub tiling_scheme: TilingScheme,
}

impl Default for ImageryConfig {
    fn default() -> Self {
        Self {
            kind: XyzTag::Xyz,
            credit: String::new(),
            subdomains: String::new(), // Martin 单机无分片，占位满足前端类型
            minimum_level: 0,          // ready 时由转换器回写真实值
            maximum_level: 18,
            tiling_scheme: TilingScheme::WebMercator,
        }
    }
}

// ---------------- 地形 ----------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerrainConfig {
    pub request_vertex_normals: bool,
    pub request_water_mask: bool,
    pub credit: String,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            request_vertex_normals: true,
            request_water_mask: false,
            credit: String::new(),
        }
    }
}
// ---------------- 点云 ----------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PointCloudStyleMode {
    #[default]
    Original,
    Uniform,
    Classification,
    Elevation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationRule {
    pub classification: i64,
    pub color: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevationStop {
    pub elevation: f64,
    pub color: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointCloudConfig {
    pub style_mode: PointCloudStyleMode,
    pub uniform_color: String,
    pub classification_rules: Vec<ClassificationRule>,
    pub elevation_stops: Vec<ElevationStop>,

    pub attenuation: bool,
    pub eye_dome_lighting: bool,

    pub maximum_screen_space_error: f64,
    pub dynamic_screen_space_error: bool,
    pub credit: String,
}

impl Default for PointCloudConfig {
    fn default() -> Self {
        Self {
            style_mode: PointCloudStyleMode::Original,
            uniform_color: "#ffffff".into(),
            classification_rules: Vec::new(),
            elevation_stops: Vec::new(),
            attenuation: true,
            eye_dome_lighting: false,
            maximum_screen_space_error: 16.0,
            dynamic_screen_space_error: false,
            credit: String::new(),
        }
    }
}
// ---------------- 模型 ----------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Shadows {
    #[default]
    Enabled,
    Disabled,
    ReceiveOnly,
    CastOnly,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MeshConfig {
    pub maximum_screen_space_error: f64,
    pub shadows: Shadows,
    pub dynamic_screen_space_error: bool,
    pub maximum_memory_usage: u32,
    pub credit: String,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            maximum_screen_space_error: 16.0,
            shadows: Shadows::Enabled,
            dynamic_screen_space_error: true,
            maximum_memory_usage: 512,
            credit: String::new(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TilesConfig {
    Las(PointCloudConfig),
    Mesh(MeshConfig),
}

// ---------------- 矢量 ----------------
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorType {
    #[default]
    Geojson,
    Kml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointStyle {
    pub color: String,
    pub size: f64,
    pub outline_color: String,
    pub outline_width: f64,
}

impl Default for PointStyle {
    fn default() -> Self {
        Self {
            color: "#ffffff".into(),
            size: 8.0,
            outline_color: "#000000".into(),
            outline_width: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PolylineStyle {
    pub color: String,
    pub width: f64,
}

impl Default for PolylineStyle {
    fn default() -> Self {
        Self {
            color: "#3388ff".into(),
            width: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PolygonStyle {
    pub color: String,
    pub outline: bool,
    pub outline_color: String,
    pub outline_width: f64,
}

impl Default for PolygonStyle {
    fn default() -> Self {
        Self {
            color: "#3388ff".into(),
            outline: true,
            outline_color: "#ffffff".into(),
            outline_width: 1.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VectorDisplay {
    #[serde(rename = "type")]
    pub kind: VectorType,
    pub clamp_to_ground: bool,
    pub point: PointStyle,
    pub polyline: PolylineStyle,
    pub polygon: PolygonStyle,
}

impl Default for VectorDisplay {
    fn default() -> Self {
        Self {
            kind: VectorType::Geojson,
            clamp_to_ground: true,
            point: PointStyle::default(),
            polyline: PolylineStyle::default(),
            polygon: PolygonStyle::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceExtra {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<TilesConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imagery: Option<ImageryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorDisplay>, // ← 新增
}

// ------------------------请求-------------------------------------

/// 四个创建接口共用的字段
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBase {
    pub name: String,
    pub source_path: String,
    pub group_name: Option<String>,
    pub epsg: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateImageryRequest {
    pub base: CreateBase,
    pub extra: ImageryConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerrainRequest {
    pub base: CreateBase,
    pub extra: TerrainConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTilesRequest {
    pub base: CreateBase,
    pub extra: TilesConfig, // 必填：type 就是分派依据，不给没法决定走哪条产线
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Imagery,
    Terrain,
    Tiles,
    Vector,
}
impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Imagery => "imagery",
            Self::Terrain => "terrain",
            Self::Tiles => "tiles",
            Self::Vector => "vector",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortStyle {
    NameAsc,
    NameDesc,
    TimeAsc,
    TimeDesc,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListRequest {
    pub category: Category,
    pub start: i64,
    pub limit: i64,
    pub sort_style: SortStyle,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResourcesRequest {
    pub ids: Vec<i64>,
}

// ------------------------响应-------------------------------------

#[derive(Debug, sqlx::FromRow)]

pub struct Resource {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub url: Option<String>,
    pub source_path: String,
    pub output_path: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub min_zoom: Option<i32>,
    pub max_zoom: Option<i32>,
    pub epsg: i32,
    pub group_name: Option<String>,
    #[sqlx(json)]
    pub extra: ResourceExtra,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFiltered {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub url: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub min_zoom: Option<i32>,
    pub max_zoom: Option<i32>,
    pub epsg: i32,
    pub group_name: Option<String>,
    pub extra: ResourceExtra,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
impl From<Resource> for ResourceFiltered {
    fn from(r: Resource) -> Self {
        Self {
            id: r.id,
            name: r.name,
            category: r.category,
            url: r.url,
            status: r.status,
            error_message: r.error_message,
            min_lon: r.min_lon,
            min_lat: r.min_lat,
            max_lon: r.max_lon,
            max_lat: r.max_lat,
            min_zoom: r.min_zoom,
            max_zoom: r.max_zoom,
            epsg: r.epsg,
            group_name: r.group_name,
            extra: r.extra,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceResponse {
    pub code: i32,
    pub id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResponse {
    pub code: i32,
    pub resources: Vec<ResourceFiltered>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResourcesResponse {
    pub code: i32,
    pub failed: Vec<i64>,
}
