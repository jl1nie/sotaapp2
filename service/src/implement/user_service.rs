use async_trait::async_trait;
use common::config::AppConfig;
use common::error::AppResult;
use shaku::Component;
use std::sync::Arc;

use domain::model::pota::{POTAAlert, POTAReference, POTASpot};
use domain::model::sota::{SOTAAlert, SOTAReference, SOTASpot};

use domain::model::common::event::{FindAct, FindAppResult, FindRef};

use crate::services::UserService;
use domain::repository::pota::{POTADatabase, POTActivationDatabase};
use domain::repository::sota::{SOTAActivationDatabase, SOTADatabase};

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_db: Arc<dyn SOTADatabase>,
    #[shaku(inject)]
    pota_db: Arc<dyn POTADatabase>,
    #[shaku(inject)]
    sota_act_db: Arc<dyn SOTAActivationDatabase>,
    #[shaku(inject)]
    pota_act_db: Arc<dyn POTActivationDatabase>,
    config: AppConfig,
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn find_reference(
        &self,
        event: FindRef,
    ) -> AppResult<FindAppResult<SOTAReference, POTAReference>> {
        let sota = if event.is_sota() {
            Some(self.sota_db.find_reference(&event).await?)
        } else {
            None
        };

        let pota = if event.is_pota() {
            Some(self.pota_db.find_reference(&event).await?)
        } else {
            None
        };

        Ok(FindAppResult { sota, pota })
    }

    async fn find_alert(&self, event: FindAct) -> AppResult<FindAppResult<SOTAAlert, POTAAlert>> {
        let sota = Some(self.sota_act_db.find_alert(&event).await?);
        let pota = Some(self.pota_act_db.find_alert(&event).await?);

        Ok(FindAppResult { sota, pota })
    }

    async fn find_spot(&self, event: FindAct) -> AppResult<FindAppResult<SOTASpot, POTASpot>> {
        let sota = Some(self.sota_act_db.find_spot(&event).await?);
        let pota = Some(self.pota_act_db.find_spot(&event).await?);

        Ok(FindAppResult { sota, pota })
    }
}
