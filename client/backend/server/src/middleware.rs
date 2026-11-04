use crate::error::AppError;
use axum::{extract::Request, middleware::Next, response::Response};
use std::sync::Arc;

pub async fn log_app_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    if let Some(err) = response.extensions().get::<Arc<AppError>>() {
        tracing::error!(?err, "unexpected error in handler");
    }

    response
}
