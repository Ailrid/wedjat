use super::handler as h;
use crate::state::AppState;
use axum::{Router, routing::post};

pub fn resources_router() -> Router<AppState> {
    Router::new()
        .route("/resources/imagery", post(h::create_imagery))
        .route("/resources/terrain", post(h::create_terrain))
        .route("/resources/tiles", post(h::create_tiles))
        .route("/resources/list", post(h::list))
        .route("/resources/delete", post(h::delete))
}
