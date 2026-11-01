use thiserror::Error;
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("Error: {0}")]
    Io(#[from] std::io::Error),
}
