use anyhow::Result;
use apalis::prelude::*;
use apalis_cron::CronStream;
use apalis_cron::Schedule;
use chrono::{DateTime, Utc};
use registry::AppState;
use std::str::FromStr;

use crate::aggregator::alerts_spots::{UpdateAlerts, UpdateSpots};
use crate::aggregator::geomag::UpdateGeoMag;

use common::config::AppConfig;

use super::summitlist::UpdateSummitList;

async fn alert_executer(job: DateTime<Utc>, svc: Data<UpdateAlerts>) {
    tracing::info!("Update Alerts {}", job);
    let _ = svc.update().await;
}

async fn spot_executer(job: DateTime<Utc>, svc: Data<UpdateSpots>) {
    tracing::info!("Update Spots {}", job);
    let _ = svc.update().await;
}

async fn geomag_executer(job: DateTime<Utc>, svc: Data<UpdateGeoMag>) {
    tracing::info!("Update geomag {}", job);
    let _ = svc.update().await;
}

async fn summitlist_executer(job: DateTime<Utc>, svc: Data<UpdateSummitList>) {
    tracing::info!("Update summitlist {}", job);
    let _ = svc.update(false).await;
}

pub async fn build(config: &AppConfig, state: &AppState) -> Result<()> {
    let alert_schedule =
        Schedule::from_str(&config.alert_update_schedule.clone()).expect("bad cron format");

    let alert = UpdateAlerts::new(config, state);
    alert.update().await?;
    let alert_job = WorkerBuilder::new("update-alerts")
        .data(alert)
        .backend(CronStream::new(alert_schedule))
        .build_fn(alert_executer);

    let spot = UpdateSpots::new(config, state);
    spot.update().await?;
    let spot_schedule = Schedule::from_str(&config.spot_update_schedule).expect("bad cron format");
    let spot_job = WorkerBuilder::new("update-spots")
        .data(spot)
        .backend(CronStream::new(spot_schedule))
        .build_fn(spot_executer);

    let geomag = UpdateGeoMag::new(config, state);
    geomag.update().await?;
    let geomag_schedule =
        Schedule::from_str(&config.geomag_update_schedule).expect("bad cron format");
    let geomag_job = WorkerBuilder::new("update-spots")
        .data(geomag)
        .backend(CronStream::new(geomag_schedule))
        .build_fn(geomag_executer);

    let summitlist = UpdateSummitList::new(config, state);
    summitlist.update(config.import_all_at_startup).await?;
    let summitlist_schedule =
        Schedule::from_str(&config.sota_summitlist_update_schedule).expect("bad cron format");
    let summit_job = WorkerBuilder::new("update-spots")
        .data(summitlist)
        .backend(CronStream::new(summitlist_schedule))
        .build_fn(summitlist_executer);

    let alert_future = Monitor::new().register(alert_job).run();
    let spot_future = Monitor::new().register(spot_job).run();
    let geomag_future = Monitor::new().register(geomag_job).run();
    let summit_future = Monitor::new().register(summit_job).run();

    let _res = tokio::join!(alert_future, spot_future, geomag_future, summit_future);
    Ok(())
}
