use crate::{config::Config, error::AppError, state::AppState};
use axum::Router;
use std::sync::Arc;

pub struct App {
    state: AppState,
    router: Router<AppState>,
}

impl App {
    pub async fn new(config_path: &str) -> Result<Self, AppError> {
        let config = Arc::new(Config::load(config_path)?);
        let state = AppState::new(config).await?;
        sqlx::migrate!("./migrations").run(&state.pool).await?;

        Ok(Self {
            state,
            router: Router::new(),
        })
    }

    pub fn with_router(mut self, router: Router<AppState>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    pub async fn run(self) -> Result<(), AppError> {
        let addr = format!(
            "{}:{}",
            self.state.config.server.host, self.state.config.server.port
        );
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let app = self.router.with_state(self.state);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
