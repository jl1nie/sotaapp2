use async_trait::async_trait;
use common::config::AppConfig;
use shaku::Component;
use std::sync::Arc;

use common::error::AppResult;

use domain::model::common::event::UpdateAct;
use domain::model::pota::{POTAAlert, POTASpot};
use domain::model::sota::{SOTAAlert, SOTASpot};
use domain::repository::{pota::POTActivationDatabase, sota::SOTAActivationDatabase};

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    sota_act_db: Arc<dyn SOTAActivationDatabase>,
    #[shaku(inject)]
    pota_act_db: Arc<dyn POTActivationDatabase>,
    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_sota_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()> {
        self.sota_act_db.update_alert(event).await?;
        Ok(())
    }

    async fn update_sota_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()> {
        self.sota_act_db.update_spot(event).await?;
        Ok(())
    }

    async fn update_pota_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()> {
        self.pota_act_db.update_alert(event).await?;
        Ok(())
    }

    async fn update_pota_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()> {
        self.pota_act_db.update_spot(event).await?;
        Ok(())
    }
}
