use crate::error::AppError;
use axum::{
    extract::FromRequest,
    response::{IntoResponse, Response},
};

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
