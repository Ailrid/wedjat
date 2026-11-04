use crate::config::Config;
use crate::pipeline::converter::Task;
use sqlx::PgPool;
use std::sync::Arc;
pub async fn las2tiles(task: Task, cfg: Arc<Config>, pool: PgPool) {
    todo!()
}
