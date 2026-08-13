use crate::extractor::errors::ExtractorError;
use crate::matcher::errors::MatcherError;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("ONNX error: {0}")]
    Onnx(#[from] ort::Error),

    // 把 OpenVino 的三个错误直接拍平
    #[error("OpenVino error: {0}")]
    Vino(String),

    #[error("OpenCV error: {0}")]
    Opencv(#[from] opencv::Error),

    #[error("Unsupported model type: {0}")]
    InvalidModel(String),
}

// 统一处理 OpenVino
impl From<openvino::SetupError> for BackendError {
    fn from(e: openvino::SetupError) -> Self {
        Self::Vino(e.to_string())
    }
}
impl From<openvino::InferenceError> for BackendError {
    fn from(e: openvino::InferenceError) -> Self {
        Self::Vino(e.to_string())
    }
}
impl From<openvino::LoadingError> for BackendError {
    fn from(e: openvino::LoadingError) -> Self {
        Self::Vino(e.to_string())
    }
}

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Extractor error: {0}")]
    ExtractorError(#[from] ExtractorError),
    #[error("Matcher error: {0}")]
    MatcherError(#[from] MatcherError),
    #[error("Minimal error: {0}")]
    MinimalError(#[from] geodesy::Error),
    #[error("Cannot find any position")]
    FindPositionError,
}
