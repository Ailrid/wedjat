use super::model::Resource;
use sqlx::PgPool;

pub async fn insert_pending(
    pool: &PgPool,
    name: &str,
    category: &str,
    source_path: &str,
    group_name: Option<&str>,
    epsg: i32,
    extra: &serde_json::Value,
) -> Result<Resource, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        "INSERT INTO resources (name, category, source_path, group_name, epsg, extra, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending')
         RETURNING *",
    )
    .bind(name)
    .bind(category)
    .bind(source_path)
    .bind(group_name)
    .bind(epsg)
    .bind(extra)
    .fetch_one(pool)
    .await
}

pub async fn get(pool: &PgPool, id: i64) -> Result<Option<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>("SELECT * FROM resources WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_by_group(pool: &PgPool, group_name: &str) -> Result<Vec<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>("SELECT * FROM resources WHERE group_name = $1 ORDER BY id DESC")
        .bind(group_name)
        .fetch_all(pool)
        .await
}
pub async fn list(
    pool: &PgPool,
    category: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        "SELECT * FROM resources
         WHERE ($1::text IS NULL OR category = $1)
           AND ($2::text IS NULL OR status = $2)
         ORDER BY id DESC",
    )
    .bind(category)
    .bind(status)
    .fetch_all(pool)
    .await
}

pub async fn latest_by_source_path(
    pool: &PgPool,
    source_path: &str,
) -> Result<Option<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        "SELECT * FROM resources WHERE source_path = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(source_path)
    .fetch_optional(pool)
    .await
}
