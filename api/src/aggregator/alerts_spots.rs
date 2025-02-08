use reqwest;
use shaku::HasComponent;
use std::sync::Arc;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use domain::model::activation::{Alert, Spot};
use registry::AppRegistry;
use service::services::AdminPeriodicService;

use crate::model::alerts::{POTAAlert, SOTAAlert};
use crate::model::spots::{POTASpot, SOTASpot};

pub async fn update_alerts(config: &AppConfig, registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();

    let endpoint = config.sota_alert_endpoint.clone();
    let response = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<SOTAAlert>>()
        .await
        .map_err(AppError::GetError)?;

    let mut requests: Vec<Alert> = response
        .into_iter()
        .filter_map(|sa| AppResult::<Alert>::from(sa).ok())
        .collect();

    let endpoint = config.pota_alert_endpoint.clone();
    let response = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<POTAAlert>>()
        .await
        .map_err(AppError::GetError)?;

    let requests_pota: Vec<Alert> = response
        .into_iter()
        .filter_map(|pa| AppResult::<Alert>::from(pa).ok())
        .collect();

    requests.extend(requests_pota);

    service.update_alerts(requests).await?;
    Ok(())
}

pub async fn update_spots(config: &AppConfig, registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();

    let endpoint = config.sota_spot_endpoint.clone();
    let response = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<SOTASpot>>()
        .await
        .map_err(AppError::GetError)?;

    let mut requests: Vec<Spot> = response
        .into_iter()
        .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
        .collect();

    let endpoint = config.pota_spot_endpoint.clone();
    let response = reqwest::get(&endpoint)
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<POTASpot>>()
        .await
        .map_err(AppError::GetError)?;

    let requests_pota: Vec<Spot> = response
        .into_iter()
        .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
        .collect();

    requests.extend(requests_pota);

    service.update_spots(requests).await?;
    Ok(())
}
