use reqwest;
use shaku::HasComponent;
use std::sync::Arc;

use common::error::AppResult;
use common::{config::AppConfig, error::AppError};
use registry::AppRegistry;
use service::{model::sota::UploadSOTACSV, services::AdminService};

pub async fn update_summit_list(config: AppConfig, registry: Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminService = registry.resolve_ref();
    let endpoint = config.sota_summitlist_endpoint.clone();

    let data = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .text()
        .await
        .map_err(AppError::GetError)?;

    let event = UploadSOTACSV { data };
    service.update_summit_list(event).await?;

    Ok(())
}
