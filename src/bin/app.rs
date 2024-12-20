use anyhow::{Error, Result};
use axum::extract::DefaultBodyLimit;
use axum::Router;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use common::config::AppConfigBuilder;

use api::handler::health::build_health_chek_routers;
use api::handler::sota::build_sota_routers;

use registry::{AppRegistry, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let config = AppConfigBuilder::default()
        .database(None)
        .sota_endpoint("https://api2.sota.org.uk/api")
        .pota_endpoint("https://api.pota.app")
        .alert_expire(Duration::from_secs(3600u64 * 48))
        .alert_schedule("0 */5 * * * *")
        .spot_expire(Duration::from_secs(3600u64 * 48))
        .spot_schedule("0 */1 * * * *")
        .build();

    let module = AppRegistry::new(&config);
    let app_state = AppState::new(module);
    let job_state = app_state.clone();

    let app = Router::new()
        .merge(build_health_chek_routers())
        .merge(build_sota_routers())
        .with_state(app_state)
        .nest_service("/", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 32));

    let addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);

    let http = async { axum::serve(listener, app).await.map_err(Error::from) };
    let job_monitor = async { api::aggregator::builder::build(&config, &job_state).await };

    let _res = tokio::join!(job_monitor, http);

    Ok(())
}
