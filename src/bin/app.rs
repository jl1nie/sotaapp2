use axum::extract::DefaultBodyLimit;
use axum::Router;
use tower_http::services::ServeDir;

use anyhow::{Error, Result};
use common::config::AppConfigBuilder;
use registry::{AppRegistry, AppState};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use web_api::handler::health::build_health_chek_routers;
use web_api::handler::sota::build_sota_routers;

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let config = AppConfigBuilder::default()
        .database(None)
        .alert_expire(Duration::from_secs(3600u64 * 48))
        .spot_expire(Duration::from_secs(3600u64 * 48))
        .build();

    let module = AppRegistry::new(config);
    let app_state = AppState::new(module);

    let app = Router::new()
        .merge(build_health_chek_routers())
        .merge(build_sota_routers())
        .with_state(app_state)
        .nest_service("/", ServeDir::new("data"))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024));

    let addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();
    let listener = TcpListener::bind(&addr).await?;

    println!("Listening on {}", addr);

    axum::serve(listener, app).await.map_err(Error::from)
}
