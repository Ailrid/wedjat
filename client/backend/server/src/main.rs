mod app;
mod config;
mod error;
mod extract;
mod middleware;
mod pipeline;
mod resources;
mod state;

use crate::{app::App, error::AppError};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let app = App::new("config.toml")
        .await?
        .with_router(resources::resources_router());

    app.run().await
}
