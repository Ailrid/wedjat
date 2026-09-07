use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("rknn error: {0}")]
    RknnError(#[from] rknn3::error::RknnError),
    #[error("opencv error: {0}")]
    OpencvError(#[from] opencv::Error),
}
