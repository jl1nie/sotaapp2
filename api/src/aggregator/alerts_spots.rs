use anyhow::Result;
use common::config::AppConfig;
use registry::{AppRegistry, AppState};
use service::services::AdminPeriodicService;
use shaku::HasComponent;
use std::sync::Arc;

#[derive(Clone)]
pub struct UpdateAlerts {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateAlerts {
    pub fn new(config: &AppConfig, state: AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.module.clone(),
        }
    }
    pub async fn update(&self) -> Result<()> {
        self.update_sota_alerts().await?;
        self.update_pota_alerts().await?;
        Ok(())
    }

    async fn update_sota_alerts(&self) -> Result<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        eprintln!("Update SOTA Alert");
        Ok(())
    }

    async fn update_pota_alerts(&self) -> Result<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        eprintln!("Update POTA Alert");
        Ok(())
    }
}

#[derive(Clone)]
pub struct UpdateSpots {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateSpots {
    pub fn new(config: &AppConfig, state: AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.module.clone(),
        }
    }
    pub async fn update(&self) -> Result<()> {
        self.update_sota_spots().await?;
        self.update_pota_spots().await?;
        Ok(())
    }

    async fn update_sota_spots(&self) -> Result<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        eprintln!("Update SOTA Spots");
        Ok(())
    }

    async fn update_pota_spots(&self) -> Result<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        eprintln!("Update POTA Spots");
        Ok(())
    }
}
