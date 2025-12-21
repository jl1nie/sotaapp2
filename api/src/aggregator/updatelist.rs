use service::model::pota::UploadPOTAReference;
use shaku::HasComponent;
use std::sync::Arc;
use std::time::Duration;

use common::error::AppResult;
use common::http;
use common::{config::AppConfig, error::AppError};
use registry::AppRegistry;
use service::{model::sota::UploadSOTASummit, services::AdminService};

pub async fn update_summit_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.sota_summitlist_endpoint.clone();

    let client = http::client();
    let data = client
        .get(&endpoint)
        .send()
        .await
        .map_err(AppError::GetError)?
        .text()
        .await
        .map_err(AppError::GetError)?;

    let event = UploadSOTASummit { data };
    service.update_summit_list(event).await?;

    if config.reboot_after_update {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tracing::info!("Sending graceful shutdown signal.");
        let _ = config.shutdown_tx.send(true);
    }
    tracing::info!("Summit list updated successfully.");
    Ok(())
}

pub async fn update_park_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.pota_parklist_endpoint.clone();

    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
    let client = http::client();
    let data = client
        .get(&endpoint)
        .header(http::header::USER_AGENT, user_agent)
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
