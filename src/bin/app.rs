use anyhow::{Error, Result};
use axum::Router;
use common::config::AppConfig;
use data_access::database::connect_database_with;
use registry::AppRegistry;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use web_api::handler::health::build_health_chek_routers;

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let app_config = AppConfig::new()?;
    let pool = connect_database_with(&app_config.database)?;
    let registry = AppRegistry::new(pool);

    let app = Router::new()
        .merge(build_health_chek_routers())
        .with_state(registry);

    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080);
    let listener = TcpListener::bind(&addr).await?;

    println!("Listening on {}", addr);

    axum::serve(listener, app).await.map_err(Error::from)
}
