use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // 4xx：用户输入问题，不记录日志
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("JSON rejection: {0}")]
    JsonRejection(#[from] JsonRejection),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Conflict: {0}")]
    ConflictError(String),

    // 5xx：服务端问题，中间件会记录
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Migrate error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Converter error: {0}")]
    Converter(#[from] crate::pipeline::error::ConverterError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            code: i32,
            message: String,
        }

        let (status, code, message, err) = match &self {
            // ---- 4xx：不插入 Extension，不记录 ----
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, 400, m.clone(), None),
            AppError::JsonRejection(rejection) => {
                (rejection.status(), 400, rejection.body_text(), None)
            }
            AppError::SerdeError(err) => (
                StatusCode::BAD_REQUEST,
                400,
                format!("JSON error: {}", err),
                None,
            ),
            AppError::ConflictError(m) => (StatusCode::CONFLICT, 400, m.clone(), None),

            // ---- 5xx：插入 Extension，交给中间件记录 ----
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                500,
                "internal error".to_string(),
                Some(self),
            ),
        };

        let mut response = (
            status,
            Json(ErrorResponse {
                code: code,
                message: message,
            }),
        )
            .into_response();

        if let Some(err) = err {
            response.extensions_mut().insert(Arc::new(err));
        }
        response
    }
}
