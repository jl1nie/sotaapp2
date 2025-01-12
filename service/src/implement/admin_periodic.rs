use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::config::AppConfig;
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
    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        tracing::info!("Update {} alerts", alerts.len());

        self.act_repo.update_alerts(alerts).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
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
