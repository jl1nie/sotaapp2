use anyhow::{Error, Result};
use axum::Router;
use common::config::AppConfigBuilder;
use std::net::SocketAddr;
use std::time::Duration;
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
        .geomag_endpoint("https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt")
        .mapcode_endpoint("https://japanmapcode.com/mapcode")
        .alert_expire(Duration::from_secs(3600u64 * 48))
        .alert_update_schedule("30 */30 * * * *")
        .spot_expire(Duration::from_secs(3600u64 * 48))
        .spot_update_schedule("0 */20 * * * *")
        .geomag_update_schedule("0 35 */3 * * * *")
        //.sota_import_association(r#"^JA\d*"#)
        .sota_import_association(r#".*"#)
        .build();

    let filter = EnvFilter::new("info");
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let module = AppRegistry::new(&config);
    let app_state = AppState::new(module);
    let job_state = app_state.clone();

    let app = Router::new()
        .merge(v2::routes())
        .with_state(app_state)
        .nest_service("/", ServeDir::new("static"));

    let addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    let http = async { axum::serve(listener, app).await.map_err(Error::from) };
    let job_monitor = async { api::aggregator::builder::build(&config, &job_state).await };

    let _res = tokio::join!(job_monitor, http);

    Ok(())
}
