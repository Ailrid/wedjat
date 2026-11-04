use crate::config::Config;
use crate::pipeline::converter::Task;
use crate::pipeline::error::ConverterError;
use sqlx::PgPool;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use uuid::Uuid;

pub async fn tiff2terrain(
    task: Task,
    cfg: Arc<Config>,
    pool: PgPool,
) -> Result<(), ConverterError> {
    todo!()
}
