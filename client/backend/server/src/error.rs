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
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("JSON rejection: {0}")]
    JsonRejection(#[from] JsonRejection),

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
            error: String,
        }

        let (status, message, err) = match &self {
            // ---- 4xx：不插入 Extension，不记录 ----
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone(), None),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone(), None),
            AppError::JsonRejection(rejection) => (rejection.status(), rejection.body_text(), None),

            // ---- 5xx：插入 Extension，交给中间件记录 ----
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
                Some(self),
            ),
        };

        let mut response = (status, Json(ErrorResponse { error: message })).into_response();
        if let Some(err) = err {
            response.extensions_mut().insert(Arc::new(err));
        }
        response
    }
}
