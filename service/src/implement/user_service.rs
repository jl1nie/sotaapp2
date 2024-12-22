use async_trait::async_trait;
use common::config::AppConfig;
use common::error::AppResult;
use shaku::Component;
use std::sync::Arc;

use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{FindAct, FindAppResult, FindRef, FindResult};

use crate::services::UserService;
use domain::repository::activation::ActivationRepositry;
use domain::repository::pota::POTAReferenceRepositry;
use domain::repository::sota::SOTAReferenceReposity;

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SOTAReferenceReposity>,
    #[shaku(inject)]
    pota_repo: Arc<dyn POTAReferenceRepositry>,
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    config: AppConfig,
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn find_reference(&self, event: FindRef) -> AppResult<FindAppResult> {
        let mut result = FindAppResult::default();

        if event.is_sota() {
            result.sota(self.sota_repo.find_reference(&event).await?)
        }
        if event.is_pota() {
            result.pota(self.pota_repo.find_reference(&event).await?)
        }
        Ok(result)
    }

    async fn find_alert(&self, event: FindAct) -> AppResult<FindResult<Alert>> {
        Ok(self.act_repo.find_alert(&event).await?)
    }

    async fn find_spot(&self, event: FindAct) -> AppResult<FindResult<Spot>> {
        Ok(self.act_repo.find_spot(&event).await?)
    }
}
