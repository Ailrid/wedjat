use crate::errors::BackendError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExtractorError {
    #[error("Model backend forward error:{0}")]
    BackendError(#[from] BackendError),
    #[error("InvalidModel:{0}")]
    ModelTypeError(String),
    #[error("Matched kpt too few:{0}")]
    TooFewMatches(usize),
    #[error("Opencv error:{0}")]
    OpencvError(#[from] opencv::Error),
    #[error("InvalidMatch")]
    InvalidMatch,
    #[error("DegenerateMatrix")]
    DegenerateMatrix,
}
