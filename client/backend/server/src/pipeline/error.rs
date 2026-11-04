use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ConverterError {
    #[error("Queue full")]
    QueueFull,

    #[error("Submit task error")]
    SubmitTaskError(#[from] mpsc::error::TrySendError<super::converter::Task>),

    #[error("Conversion failed")]
    CommandFailed { cmd: String, stderr: String },

    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("SQL error")]
    SQLError(#[from] sqlx::Error),
}
