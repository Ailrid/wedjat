use super::handler as h;
use crate::state::AppState;
use axum::{Router, routing::post};

pub fn resources_router() -> Router<AppState> {
    Router::new()
        .route("/resources/create", post(h::create))
        .route("/resources/get", post(h::get))
        .route("/resources/group", post(h::get_group))
        .route("/resources/list", post(h::list))
}
