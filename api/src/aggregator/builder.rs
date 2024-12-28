use anyhow::Result;
use apalis::prelude::*;
use apalis_cron::CronStream;
use apalis_cron::Schedule;
use chrono::{DateTime, Utc};
use registry::AppState;
use std::str::FromStr;

use crate::aggregator::alerts_spots::{UpdateAlerts, UpdateSpots};
use common::config::AppConfig;

async fn alert_executer(job: DateTime<Utc>, svc: Data<UpdateAlerts>) {
    tracing::info!("Update Alerts {}", job);
    let _ = svc.update().await;
}

async fn spot_executer(job: DateTime<Utc>, svc: Data<UpdateSpots>) {
    tracing::info!("Update Spots {}", job);
    let _ = svc.update().await;
}

pub async fn build(config: &AppConfig, state: &AppState) -> Result<()> {
    let alert_schedule =
        Schedule::from_str(&config.alert_update_schedule.clone()).expect("bad cron format");
    let alert_job = WorkerBuilder::new("update-alerts")
        .data(UpdateAlerts::new(config, state))
        .backend(CronStream::new(alert_schedule))
        .build_fn(alert_executer);

    let spot_schedule = Schedule::from_str(&config.spot_update_schedule).expect("bad cron format");
    let spot_job = WorkerBuilder::new("update-spots")
        .data(UpdateSpots::new(config, state))
        .backend(CronStream::new(spot_schedule))
        .build_fn(spot_executer);

    let a = Monitor::new().register(alert_job).run();
    let b = Monitor::new().register(spot_job).run();

    let _res = tokio::join!(a, b);
    Ok(())
}
