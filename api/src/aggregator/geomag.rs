use anyhow::Result;
use common::config::AppConfig;
use reqwest;
use shaku::HasComponent;
use std::sync::Arc;

use registry::{AppRegistry, AppState};
use service::services::AdminPeriodicService;

#[derive(Clone)]
pub struct UpdateGeomag {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateGeomag {
    pub fn new(config: &AppConfig, state: &AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.into(),
        }
    }

    pub async fn update(&self) -> Result<()> {
        todo!()
        /*
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();

        let endpoint = self.config.sota_alert_endpoint.clone();
        let response = reqwest::get(&endpoint)
            .await?
            .json::<Vec<SOTAAlert>>()
            .await?;

        let requests: Vec<Alert> = response
            .into_iter()
            .filter_map(|sa| Result::<Alert>::from(sa).ok())
            .collect();

        service.update_alerts(requests).await?;

        let endpoint = self.config.pota_alert_endpoint.clone();
        let response = reqwest::get(&endpoint)
            .await?
            .json::<Vec<POTAAlert>>()
            .await?;

        let requests: Vec<Alert> = response
            .into_iter()
            .filter_map(|pa| Result::<Alert>::from(pa).ok())
            .collect();

        service.update_alerts(requests).await?;
        Ok(())
        */
    }
}
