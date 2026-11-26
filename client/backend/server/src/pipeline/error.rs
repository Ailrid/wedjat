use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("Queue full")]
    QueueFull,

    #[error("Submit task error: {0}")]
    SubmitTaskError(#[from] mpsc::error::TrySendError<super::converter::Task>),

    #[error("command `{cmd}` failed: {stderr}")]
    CommandFailed { cmd: String, stderr: String },

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("SQL error: {0}")]
    SQLError(#[from] sqlx::Error),

    #[error("{0}")]
    OtherError(String),
}
