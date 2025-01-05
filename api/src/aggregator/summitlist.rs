use reqwest;
use shaku::HasComponent;
use std::sync::Arc;

use common::error::AppResult;
use common::{config::AppConfig, error::AppError};
use registry::{AppRegistry, AppState};
use service::{model::sota::UploadSOTACSV, services::AdminService};

#[derive(Clone)]
pub struct UpdateSummitList {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateSummitList {
    pub fn new(config: &AppConfig, state: &AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.into(),
        }
    }

    pub async fn update(&self, import_all: bool) -> AppResult<()> {
        let service: &dyn AdminService = self.registry.resolve_ref();
        let endpoint = self.config.sota_summitlist_endpoint.clone();

        let data = reqwest::get(&endpoint)
            .await
            .map_err(AppError::GetError)?
            .text()
            .await
            .map_err(AppError::GetError)?;

        let event = UploadSOTACSV { data };
        if import_all {
            service.import_summit_list(event).await?;
        } else {
            service.update_summit_list(event).await?;
        }
        Ok(())
    }
}
