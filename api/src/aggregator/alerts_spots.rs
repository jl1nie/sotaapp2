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
    let mut requests: Vec<Alert> = match client.get(&endpoint).send().await {
        Err(e) => {
            tracing::error!("Failed to fetch SOTA alerts: {:?}", e);
            vec![]
        }
        Ok(resp) => match resp.json::<Vec<SotaAlert>>().await {
            Err(e) => {
                tracing::error!("Failed to parse SOTA alerts: {:?}", e);
                vec![]
            }
            Ok(sota_alerts) => {
                let total = sota_alerts.len();
                let converted: Vec<Alert> = sota_alerts
                    .into_iter()
                    .filter_map(|sa| match AppResult::<Alert>::from(sa) {
                        Ok(a) => Some(a),
                        Err(e) => {
                            tracing::warn!("Failed to convert SOTA alert: {:?}", e);
                            None
                        }
                    })
                    .collect();
                tracing::info!(
                    "Fetched SOTA alerts: {}/{} converted",
                    converted.len(),
                    total
                );
                converted
            }
        },
    };

    let endpoint = config.pota_alert_endpoint.clone();
    match client.get(&endpoint).send().await {
        Err(e) => {
            tracing::error!("Failed to fetch POTA alerts: {:?}", e);
        }
        Ok(resp) => match resp.json::<Vec<PotaAlert>>().await {
            Err(e) => {
                tracing::error!("Failed to parse POTA alerts: {:?}", e);
            }
            Ok(pota_alerts) => {
                let total = pota_alerts.len();
                let converted: Vec<Alert> = pota_alerts
                    .into_iter()
                    .filter_map(|pa| match AppResult::<Alert>::from(pa) {
                        Ok(a) => Some(a),
                        Err(e) => {
                            tracing::warn!("Failed to convert POTA alert: {:?}", e);
                            None
                        }
                    })
                    .collect();
                tracing::info!(
                    "Fetched POTA alerts: {}/{} converted",
                    converted.len(),
                    total
                );
                requests.extend(converted);
            }
        },
    }

    tracing::info!("Updating {} alerts total.", requests.len());

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
