use super::model::{Resource, ResourceExtra};
use crate::error::AppError;
use sqlx::{PgPool, Postgres, QueryBuilder};

/// 统一的插入。url / output_path 留 NULL，由 converter 的 prepare 填
pub async fn insert(
    pool: &PgPool,
    name: &str,
    category: &str,
    source_path: &str,
    group_name: Option<&str>,
    epsg: i32,
    extra: &ResourceExtra,
) -> Result<Resource, AppError> {
    let extra = serde_json::to_value(extra)?;

    let row = sqlx::query_as::<_, Resource>(
        "INSERT INTO resources (name, category, source_path, group_name, epsg, extra, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending')
         RETURNING *",
    )
    .bind(name)
    .bind(category)
    .bind(source_path)
    .bind(group_name)
    .bind(epsg)
    .bind(&extra)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// 同一源文件是否正在转换中，去重用
pub async fn active_by_source_path(
    pool: &PgPool,
    source_path: &str,
) -> Result<Option<Resource>, AppError> {
    let row = sqlx::query_as::<_, Resource>(
        "SELECT * FROM resources
          WHERE source_path = $1
            AND status IN ('pending', 'converting')
          ORDER BY id DESC
          LIMIT 1",
    )
    .bind(source_path)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// 入队失败兜底：别让行永远停在 pending
pub async fn mark_failed(pool: &PgPool, id: i64, message: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE resources
            SET status = 'failed', error_message = $2, updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(message)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_ready(
    pool: &PgPool,
    category: &str,
    order_by: &'static str,
    start: i64,
    limit: i64,
) -> Result<(i64, Vec<Resource>), AppError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources WHERE category = $1 AND status = 'ready'",
    )
    .bind(category)
    .fetch_one(pool)
    .await?;

    if total == 0 {
        return Ok((0, Vec::new()));
    }

    let mut qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM resources WHERE category = ");

    qb.push_bind(category)
        .push(" AND status = 'ready' ORDER BY ")
        .push(order_by) // &'static str，来源只有四个字面量
        .push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(start);

    let rows = qb.build_query_as::<Resource>().fetch_all(pool).await?;

    Ok((total, rows))
}

/// 批量删，只删已结束的。返回真正删掉的 (id, output_path)
pub async fn delete_finished(
    pool: &PgPool,
    ids: &[i64],
) -> Result<Vec<(i64, Option<String>)>, AppError> {
    let rows = sqlx::query_as::<_, (i64, Option<String>)>(
        "DELETE FROM resources
          WHERE id = ANY($1) AND status IN ('ready', 'failed')
          RETURNING id, output_path",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
