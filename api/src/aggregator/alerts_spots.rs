use common::config::AppConfig;
use common::error::{AppError, AppResult};
use reqwest;

use crate::model::alerts::{POTAAlert, SOTAAlert};
use crate::model::spots::{POTASpot, SOTASpot};
use domain::model::common::activation::{Alert, Spot};

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
    pub fn new(config: &AppConfig, state: &AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.into(),
        }
    }

    pub async fn update(&self) -> AppResult<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();

        let endpoint = self.config.sota_alert_endpoint.clone();

        let response = reqwest::get(&endpoint)
            .await
            .map_err(AppError::GetError)?
            .json::<Vec<SOTAAlert>>()
            .await
            .map_err(AppError::GetError)?;

        let requests: Vec<Alert> = response
            .into_iter()
            .filter_map(|sa| AppResult::<Alert>::from(sa).ok())
            .collect();

        service.update_alerts(requests).await?;

        let endpoint = self.config.pota_alert_endpoint.clone();
        let response = reqwest::get(&endpoint)
            .await
            .map_err(AppError::GetError)?
            .json::<Vec<POTAAlert>>()
            .await
            .map_err(AppError::GetError)?;

        let requests: Vec<Alert> = response
            .into_iter()
            .filter_map(|pa| AppResult::<Alert>::from(pa).ok())
            .collect();

        service.update_alerts(requests).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct UpdateSpots {
    config: AppConfig,
    registry: Arc<AppRegistry>,
}

impl UpdateSpots {
    pub fn new(config: &AppConfig, state: &AppState) -> Self {
        Self {
            config: config.clone(),
            registry: state.into(),
        }
    }

    pub async fn update(&self) -> AppResult<()> {
        let service: &dyn AdminPeriodicService = self.registry.resolve_ref();
        let endpoint = self.config.sota_spot_endpoint.clone();
        let response = reqwest::get(&endpoint)
            .await
            .map_err(AppError::GetError)?
            .json::<Vec<SOTASpot>>()
            .await
            .map_err(AppError::GetError)?;

        let requests: Vec<Spot> = response
            .into_iter()
            .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
            .collect();

        service.update_spots(requests).await?;

        let endpoint = self.config.pota_spot_endpoint.clone();
        let response = reqwest::get(&endpoint)
            .await
            .map_err(AppError::GetError)?
            .json::<Vec<POTASpot>>()
            .await
            .map_err(AppError::GetError)?;

        let requests: Vec<Spot> = response
            .into_iter()
            .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
            .collect();

        service.update_spots(requests).await?;
        Ok(())
    }
}
