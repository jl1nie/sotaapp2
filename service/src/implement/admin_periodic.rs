use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use common::config::AppConfig;
use domain::model::common::AwardProgram;
use domain::repository::aprs::AprsRepositry;
use shaku::Component;
use std::sync::Arc;

use common::error::AppResult;

use domain::model::common::{activation::Alert, activation::Spot, event::DeleteAct};
use domain::model::geomag::GeomagIndex;
use domain::repository::{activation::ActivationRepositry, geomag::GeoMagRepositry};

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    geomag_repo: Arc<dyn GeoMagRepositry>,
    #[shaku(inject)]
    aprs_repo: Arc<dyn AprsRepositry>,
    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        tracing::info!("Update {} alerts", alerts.len());

        let now: DateTime<Utc> = Utc::now();
        let alert_window_start = now - TimeDelta::hours(3);
        let alert_window_end = now + TimeDelta::hours(5);
        let buddy: Vec<_> = alerts
            .iter()
            .filter(|a| {
                a.program == AwardProgram::SOTA
                    && a.start_time > alert_window_start
                    && a.start_time < alert_window_end
            })
            .map(|a| a.operator.clone() + "-*")
            .collect();

        if buddy.len() > 0 {
            tracing::info!("buddy list ={:?}", buddy);
            self.aprs_repo.set_buddy_list(buddy).await?;
        }

        self.act_repo.update_alerts(alerts).await?;

        let expire = now - self.config.alert_expire;
        self.act_repo
            .delete_alerts(DeleteAct { before: expire })
            .await?;

        Ok(())
    }

    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()> {
        tracing::info!("Update {} spots", spots.len());

        self.act_repo.update_spots(spots).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_spots(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn update_geomag(&self, index: GeomagIndex) -> AppResult<()> {
        self.geomag_repo.update_geomag(index).await?;
        Ok(())
    }
}
