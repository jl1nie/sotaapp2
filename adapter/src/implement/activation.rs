use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};

use crate::database::ConnectionPool;
use domain::repository::activation::ActivationRepositry;

#[derive(Component)]
#[shaku(interface = ActivationRepositry)]
pub struct ActivationRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl ActivationRepositry for ActivationRepositryImpl {
    async fn update_alert(&self, event: UpdateAct<Alert>) -> AppResult<()> {
        eprintln!("Update SOTA alerts for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<Alert>> {
        eprintln!("Find SOTA alerts with {:?} ", event);
        todo!()
    }
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete SOTA alerts with {:?} ", event);
        Ok(())
    }
    async fn update_spot(&self, event: UpdateAct<Spot>) -> AppResult<()> {
        eprintln!("Update SOTA spots for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<Spot>> {
        eprintln!("Find SOTA spots with {:?} ", event);
        todo!()
    }
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete SOTA spots with {:?} ", event);
        Ok(())
    }
}
