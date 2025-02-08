use std::sync::Arc;
use tokio::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::aggregator::alerts_spots::{update_alerts, update_spots};
use crate::aggregator::summitlist::update_summit_list;
use common::config::AppConfig;
use common::error::{AppError, AppResult};
use registry::{AppRegistry, AppState};

use super::aprs_packet::process_incoming_packet;

pub async fn build(config: &AppConfig, state: &AppState) -> AppResult<()> {
    let registry: Arc<AppRegistry> = state.into();

    let alert_interval = Duration::from_secs(config.alert_update_interval);
    let spot_interval = Duration::from_secs(config.spot_update_interval);
    let sched = JobScheduler::new().await.map_err(AppError::CronjobError)?;

    let config_alert = config.clone();
    let registry_alert = registry.clone();
    let alert_handle = tokio::spawn(async move {
        loop {
            let _ = update_alerts(&config_alert, &registry_alert).await;
            tokio::time::sleep(alert_interval).await;
        }
    });

    let config_spot = config.clone();
    let registry_spot = registry.clone();
    let spot_handle = tokio::spawn(async move {
        loop {
            let _ = update_spots(&config_spot, &registry_spot).await;
            tokio::time::sleep(spot_interval).await;
        }
    });

    let registry_aprs = registry.clone();
    let aprs_handle = tokio::spawn(async move {
        loop {
            let _ = process_incoming_packet(&registry_aprs).await;
        }
    });

    let schedule = config.sota_summitlist_update_schedule.clone();
    let config_summit = config.clone();
    sched
        .add(
            Job::new_async(&schedule, move |_uuid, _l| {
                let config = config_summit.clone();
                let registry = registry.clone();
                Box::pin(async move {
                    update_summit_list(config, registry).await.unwrap();
                })
            })
            .unwrap_or_else(|_| panic!("Bad cron format: {}", &schedule)),
        )
        .await
        .map_err(AppError::CronjobError)?;

    let _res = tokio::join!(alert_handle, spot_handle, aprs_handle);
    Ok(())
}
