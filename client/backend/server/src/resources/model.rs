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
    pub group: Option<String>,
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
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub min_zoom: Option<i32>,
    pub max_zoom: Option<i32>,
    pub group: Option<String>,
    pub extra: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}


#[derive(Debug, Deserialize)]
pub struct CreateResource {
    pub name: String,
    pub source_path: String,
    pub group: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetResource {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetResourceGroup {
    pub group_name: String,
}


