use shaku::HasComponent;
use std::sync::Arc;

use common::config::AppConfig;
use common::error::AppResult;
use common::http;
use domain::model::activation::{Alert, Spot};
use registry::AppRegistry;
use service::services::AdminPeriodicService;

use super::spot_parse::{pota_spots_from_body, sota_spots_from_body};
use crate::model::alerts::{PotaAlert, SotaAlert};

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

/// 取得元ごとに失敗を分離する。片方が失敗しても、もう片方のスポットは更新する。
async fn fetch_body(client: &reqwest::Client, endpoint: &str, label: &str) -> Option<Vec<u8>> {
    let resp = match client.get(endpoint).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch {} spots: {:?}", label, e);
            return None;
        }
    };
    match resp.bytes().await {
        Ok(b) => Some(b.to_vec()),
        Err(e) => {
            tracing::error!("Failed to read {} spots: {:?}", label, e);
            None
        }
    }
}

pub async fn update_spots(config: &AppConfig, registry: &Arc<AppRegistry>) -> AppResult<()> {
    let service: &dyn AdminPeriodicService = registry.resolve_ref();
    let client = http::client();

    let mut requests: Vec<Spot> = vec![];

    // 旧スポット API は廃止済み。SOTA 側の新しい取得方法が決まるまで取得しない(SOTA_SPOT_ENABLED=true で有効化)
    if config.sota_spot_enabled {
        if let Some(body) = fetch_body(client, &config.sota_spot_endpoint, "SOTA").await {
            let spots = sota_spots_from_body(&body);
            tracing::info!("Fetched SOTA spots: {}", spots.len());
            requests.extend(spots);
        }
    } else {
        tracing::debug!("SOTA spot update is disabled");
    }

    if let Some(body) = fetch_body(client, &config.pota_spot_endpoint, "POTA").await {
        let spots = pota_spots_from_body(&body);
        tracing::info!("Fetched POTA spots: {}", spots.len());
        requests.extend(spots);
    }

    tracing::info!("Updating {} spots total.", requests.len());

    service.update_spots(requests).await?;
    Ok(())
}
