use axum::Router;

use crate::state::AppState;

pub fn resources_router() -> Router<AppState> {
    let router = Router::new();

    Router::new().nest("/api/resources", router)
}
