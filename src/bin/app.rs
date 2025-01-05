use anyhow::{Error, Result};
use axum::Router;
use chrono::Duration;
use common::config::AppConfigBuilder;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use api::handler::v2;
use registry::{AppRegistry, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap().await
}

async fn bootstrap() -> Result<()> {
    let config = AppConfigBuilder::default()
        .database(None)
        .sota_alert_endpoint("https://api2.sota.org.uk/api/alerts")
        .sota_spot_endpoint("https://api2.sota.org.uk/api/spots/20?")
        .pota_alert_endpoint("https://api.pota.app/activation/")
        .pota_spot_endpoint("https://api.pota.app/spot/activator/")
        .sota_summitlist_endpoint("https://www.sotadata.org.uk/summitslist.csv")
        .sota_summitlist_update_schedule("0 0 16 * * * *")
        .import_all_at_startup(false)
        .geomag_endpoint("https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt")
        .geomag_update_schedule("0 35 */3 * * * *")
        .mapcode_endpoint("https://japanmapcode.com/mapcode")
        .alert_expire(Duration::hours(48))
        .alert_update_schedule("30 */10 * * * *")
        .spot_expire(Duration::hours(24))
        .spot_update_schedule("0 */2 * * * *")
        .log_expire(Duration::days(180))
        .build();

    let filter = EnvFilter::new("info");
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let module = AppRegistry::new(&config);
    let app_state = AppState::new(module);
    let job_state = app_state.clone();

    let app = Router::new()
        .merge(v2::routes())
        .with_state(app_state)
        .nest_service("/admin-console", ServeDir::new("static"));

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    let http = async { axum::serve(listener, app).await.map_err(Error::from) };
    let job_monitor = async { api::aggregator::builder::build(&config, &job_state).await };

    let _res = tokio::join!(job_monitor, http);

    Ok(())
}
