use serde::{Deserialize, Serialize};
use sqlx::types::chrono;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Resource {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub url: Option<String>,
    pub source_path: String,
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
    pub extra: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ResourceResponse {
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
    pub extra: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Resource> for ResourceResponse {
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

#[derive(Debug, Deserialize)]
pub struct CreateResource {
    pub name: String,
    pub source_path: String,
    pub group_name: Option<String>,
    pub epsg: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetResource {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetResourceGroup {
    pub group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListResources {
    pub category: Option<String>,
    pub status: Option<String>,
}
