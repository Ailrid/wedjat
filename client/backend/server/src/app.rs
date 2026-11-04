use crate::{config::Config, error::AppError, middleware::log_app_errors, state::AppState};
use axum::extract::{MatchedPath, Request};
use axum::{Router, middleware::from_fn};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt};

pub struct App {
    state: AppState,
    router: Router<AppState>,
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl App {
    pub async fn new(config_path: &str) -> Result<Self, AppError> {
        let config = Arc::new(Config::load(config_path)?);

        let file_appender = tracing_appender::rolling::daily(&config.logging.directory, "app.log");

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::new(&config.logging.level))
            .with(fmt::Layer::new().with_writer(std::io::stdout))
            .with(fmt::Layer::new().with_writer(non_blocking));

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global subscriber");

        let state = AppState::new(config).await?;
        sqlx::migrate!("./migrations").run(&state.pool).await?;

        Ok(Self {
            state,
            router: Router::new(),
            _guard,
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
        let app = self
            .router
            .with_state(self.state)
            .layer(from_fn(log_app_errors)) // 内层
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|req: &Request| {
                        let method = req.method();
                        let uri = req.uri();
                        let matched_path =
                            req.extensions().get::<MatchedPath>().map(|p| p.as_str());
                        tracing::debug_span!("request", %method, %uri, matched_path)
                    })
                    .on_failure(()),
            ); // 关掉 5xx 自动日志，由 log_app_errors 接管;

        axum::serve(listener, app).await?;
        Ok(())
    }
}
