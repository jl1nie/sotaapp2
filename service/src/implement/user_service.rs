use async_trait::async_trait;
use common::config::AppConfig;
use common::error::AppResult;
use shaku::Component;
use std::sync::Arc;

use domain::model::pota::{POTAAlert, POTAReference, POTASpot};
use domain::model::sota::{SOTAAlert, SOTAReference, SOTASpot};

use domain::model::common::event::{FindAct, FindAppResult, FindRef};

use crate::services::UserService;
use domain::repository::pota::{POTAActivationRepositry, POTAReferenceRepositry};
use domain::repository::sota::{SOTAActivationRepositry, SOTAReferenceReposity};

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SOTAReferenceReposity>,
    #[shaku(inject)]
    pota_repo: Arc<dyn POTAReferenceRepositry>,
    #[shaku(inject)]
    sota_act_repo: Arc<dyn SOTAActivationRepositry>,
    #[shaku(inject)]
    pota_act_repo: Arc<dyn POTAActivationRepositry>,
    config: AppConfig,
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn find_reference(
        &self,
        event: FindRef,
    ) -> AppResult<FindAppResult<SOTAReference, POTAReference>> {
        let sota = if event.is_sota() {
            Some(self.sota_repo.find_reference(&event).await?)
        } else {
            None
        };

        let pota = if event.is_pota() {
            Some(self.pota_repo.find_reference(&event).await?)
        } else {
            None
        };

        Ok(FindAppResult { sota, pota })
    }

    async fn find_alert(&self, event: FindAct) -> AppResult<FindAppResult<SOTAAlert, POTAAlert>> {
        let sota = Some(self.sota_act_repo.find_alert(&event).await?);
        let pota = Some(self.pota_act_repo.find_alert(&event).await?);

        Ok(FindAppResult { sota, pota })
    }

    async fn find_spot(&self, event: FindAct) -> AppResult<FindAppResult<SOTASpot, POTASpot>> {
        let sota = Some(self.sota_act_repo.find_spot(&event).await?);
        let pota = Some(self.pota_act_repo.find_spot(&event).await?);

        Ok(FindAppResult { sota, pota })
    }
}
