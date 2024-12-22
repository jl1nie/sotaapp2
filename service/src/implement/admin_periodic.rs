use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::config::AppConfig;
use shaku::Component;
use std::sync::Arc;

use common::error::AppResult;

use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{DeleteAct, UpdateAct};
use domain::repository::activation::ActivationRepositry;

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alert(&self, event: UpdateAct<Alert>) -> AppResult<()> {
        self.act_repo.update_alert(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_alert(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn update_spot(&self, event: UpdateAct<Spot>) -> AppResult<()> {
        self.act_repo.update_spot(event).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_spot(DeleteAct { before: expire })
            .await?;
        Ok(())
    }
}
