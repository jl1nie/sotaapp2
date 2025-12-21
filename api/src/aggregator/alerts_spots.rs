use shaku::HasComponent;
use std::sync::Arc;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use common::http;
use domain::model::activation::{Alert, Spot};
use registry::AppRegistry;
use service::services::AdminPeriodicService;

use crate::model::alerts::{PotaAlert, SotaAlert};
use crate::model::spots::{PotaSpot, SotaSpot};

pub async fn update_alerts(config: &AppConfig, registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();
    let client = http::client();

    let endpoint = config.sota_alert_endpoint.clone();
    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<SotaAlert>>()
        .await
        .map_err(AppError::GetError)?;

    let mut requests: Vec<Alert> = response
        .into_iter()
        .filter_map(|sa| AppResult::<Alert>::from(sa).ok())
        .collect();

    let endpoint = config.pota_alert_endpoint.clone();
    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<PotaAlert>>()
        .await
        .map_err(AppError::GetError)?;

    let requests_pota: Vec<Alert> = response
        .into_iter()
        .filter_map(|pa| AppResult::<Alert>::from(pa).ok())
        .collect();

    requests.extend(requests_pota);

    tracing::info!("Update {} alerts.", requests.len());

    service.update_alerts(requests).await?;
    Ok(())
}

pub async fn update_spots(config: &AppConfig, registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();
    let client = http::client();

    let endpoint = config.sota_spot_endpoint.clone();
    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<SotaSpot>>()
        .await
        .map_err(AppError::GetError)?;

    let mut requests: Vec<Spot> = response
        .into_iter()
        .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
        .collect();

    let endpoint = config.pota_spot_endpoint.clone();
    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(AppError::GetError)?
        .json::<Vec<PotaSpot>>()
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
