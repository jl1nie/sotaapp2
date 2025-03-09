use anyhow::{Error, Result};
use axum::{http::HeaderValue, Router};
use common::config::AppConfig;
use firebase_auth_sdk::FireAuth;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{net::TcpListener, sync::watch};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing_subscriber::EnvFilter;

use adapter::{
    aprs::connect_aprsis_with, database::connect::connect_database_with,
    geomag::connect_geomag_with, minikvs::MiniKvs,
};
use api::handler::v2;
use registry::{AppRegistry, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let config = AppConfig::new()?;

    let filter = EnvFilter::new(&config.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .init();

    let pool = connect_database_with(&config).await?;
    let aprs = connect_aprsis_with(&config).await?;
    let geomag = connect_geomag_with(&config).await?;
    let minikvs = Arc::new(MiniKvs::new(config.auth_token_ttl));
    let module = AppRegistry::new(&config, pool, aprs, geomag, minikvs);
    let app_state = AppState::new(module);
    let job_state = app_state.clone();

    let firebase = FireAuth::new(config.firebase_api_key.clone());

    let cors = match config.cors_origin.clone() {
        Some(origin) => CorsLayer::new()
            .allow_origin(origin.parse::<HeaderValue>().unwrap())
            .allow_headers(Any)
            .allow_methods(Any),
        None => CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any),
    };

    let app = Router::new()
        .merge(v2::routes(firebase))
        .with_state(app_state)
        .layer(cors)
        .nest_service("/admin-console", ServeDir::new("static"));

    let ip_addr: IpAddr = config.host.parse().expect("Invalid IP Address");
    let addr = SocketAddr::new(ip_addr, config.port);
    let listener = TcpListener::bind(&addr).await?;
    let http = async {
        axum::serve(listener, app)
            .with_graceful_shutdown(shudown_signal(config.shutdown_rx.clone()))
            .await
            .map_err(Error::from)
    };
    let job_monitor = async { api::aggregator::builder::build(&config, &job_state).await };

    tracing::info!("DATABASE_URL = {}", config.database);
    tracing::info!("Listening on {}", addr);

    let _res = tokio::join!(job_monitor, http);

    Ok(())
}

async fn shudown_signal(mut shutdown_rx: watch::Receiver<bool>) {
    let _ = shutdown_rx.changed().await;
    tracing::info!("Shutdowm axum server.");
}
