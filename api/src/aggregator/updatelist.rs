use reqwest::{self, header};
use service::model::pota::UploadPOTAReference;
use shaku::HasComponent;
use std::sync::Arc;
use std::time::Duration;

use common::error::AppResult;
use common::{config::AppConfig, error::AppError};
use registry::AppRegistry;
use service::{model::sota::UploadSOTASummit, services::AdminService};

pub async fn update_summit_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.sota_summitlist_endpoint.clone();

    let data = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .text()
        .await
        .map_err(AppError::GetError)?;

    let event = UploadSOTASummit { data };
    service.update_summit_list(event).await?;

    tokio::time::sleep(Duration::from_secs(10)).await;
    tracing::info!("Sending graceful shutdown signal.");
    let _ = config.shutdown_tx.send(true);

    Ok(())
}

pub async fn update_park_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.pota_parklist_endpoint.clone();

    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
    let client = reqwest::Client::new();
    let data = client
        .get(&endpoint)
        .header(header::USER_AGENT, user_agent)
        .send()
        .await
        .map_err(AppError::GetError)?
        .text()
        .await
        .map_err(AppError::GetError)?;

    let event = UploadPOTAReference { data };
    service.import_pota_park_list(event).await?;

    Ok(())
}
