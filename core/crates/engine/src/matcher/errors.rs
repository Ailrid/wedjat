use thiserror::Error;

use crate::errors::BackendError;
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Clent error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Covnert btyes to Mat error: {0}")]
    ConvertError(#[from] opencv::Error),
}

#[derive(Error, Debug)]
pub enum MatcherError {
    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),
    #[error("Model backend forward error:{0}")]
    BackendError(#[from] BackendError),
    #[error("InvalidModel:{0}")]
    ModelTypeError(String),
}
