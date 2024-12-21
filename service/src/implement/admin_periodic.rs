use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::config::AppConfig;
use shaku::Component;
use std::sync::Arc;

use common::error::AppResult;

use domain::model::common::event::{DeleteAct, UpdateAct};
use domain::model::pota::{POTAAlert, POTASpot};
use domain::model::sota::{SOTAAlert, SOTASpot};
use domain::repository::pota::POTAActivationRepositry;
use domain::repository::sota::SOTAActivationRepositry;

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    sota_act_repo: Arc<dyn SOTAActivationRepositry>,
    #[shaku(inject)]
    pota_act_repo: Arc<dyn POTAActivationRepositry>,
    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_sota_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()> {
        self.sota_act_repo.update_alert(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.sota_act_repo
            .delete_alert(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn update_sota_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()> {
        self.sota_act_repo.update_spot(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.sota_act_repo
            .delete_spot(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn update_pota_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()> {
        self.pota_act_repo.update_alert(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.pota_act_repo
            .delete_alert(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn update_pota_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()> {
        self.pota_act_repo.update_spot(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.pota_act_repo
            .delete_spot(DeleteAct { before: expire })
            .await?;
        Ok(())
    }
}
