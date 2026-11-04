use crate::{config::Config, error::AppError, pipeline::converter::Converter};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub converter: Converter,
}

impl AppState {
    pub async fn new(config: Arc<Config>) -> Result<Self, AppError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .connect(&config.database.url)
            .await?;
        let converter = Converter::new(config.clone(), pool.clone());
        Ok(Self {
            pool,
            config,
            converter,
        })
    }
}
