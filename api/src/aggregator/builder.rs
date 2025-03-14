use std::sync::Arc;
use tokio::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::aggregator::alerts_spots::{update_alerts, update_spots};
use crate::aggregator::updatelist::{update_park_list, update_summit_list};
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
    let shutdown = config_alert.shutdown_rx.clone();

    let alert_handle = tokio::spawn(async move {
        'outer: loop {
            let _ = update_alerts(&config_alert, &registry_alert).await;

            for _ in 0..30 {
                if *shutdown.borrow() {
                    tracing::info!("Shutdown alert update job");
                    break 'outer;
                }
                tokio::time::sleep(alert_interval / 30).await;
            }
        }
    });

    let config_spot = config.clone();
    let registry_spot = registry.clone();
    let shutdown = config_spot.shutdown_rx.clone();

    let spot_handle = tokio::spawn(async move {
        'outer: loop {
            if let Err(e) = update_spots(&config_spot, &registry_spot).await {
                tracing::error!("Update Spot Error {:?}", e);
            }

            for _ in 0..10 {
                if *shutdown.borrow() {
                    tracing::info!("Shutdown spot update job");
                    break 'outer;
                }
                tokio::time::sleep(spot_interval / 10).await;
            }
        }
    });

    let registry_aprs = registry.clone();
    let _aprs_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = process_incoming_packet(&registry_aprs).await {
                tracing::error!("APRS Error {:?}", e);
            }
        }
    });

    let schedule = config.sota_summitlist_update_schedule.clone();
    let config_summit = config.clone();
    let registry_summit = registry.clone();
    sched
        .add(
            Job::new_async(&schedule, move |_uuid, _l| {
                let config = config_summit.clone();
                let registry = registry_summit.clone();
                Box::pin(async move {
                    if let Err(e) = update_summit_list(config, registry).await {
                        tracing::error!("Update Summit Error {:?}", e);
                    }
                })
            })
            .unwrap_or_else(|_| panic!("Bad cron format: {}", &schedule)),
        )
        .await
        .map_err(AppError::CronjobError)?;

    let schedule = config.pota_parklist_update_schedule.clone();
    let config_pota = config.clone();
    sched
        .add(
            Job::new_async(&schedule, move |_uuid, _l| {
                let config = config_pota.clone();
                let registry = registry.clone();
                Box::pin(async move {
                    if let Err(e) = update_park_list(config, registry).await {
                        tracing::error!("Update Park Error {:?}", e);
                    }
                })
            })
            .unwrap_or_else(|_| panic!("Bad cron format: {}", &schedule)),
        )
        .await
        .map_err(AppError::CronjobError)?;

    sched.start().await.map_err(AppError::CronjobError)?;

    let _res = tokio::join!(alert_handle, spot_handle);
    Ok(())
}
