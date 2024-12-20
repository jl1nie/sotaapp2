use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};
use domain::model::sota::{SOTAAlert, SOTASpot};

use crate::database::ConnectionPool;
use domain::repository::sota::SOTAActivationRepositry;

#[derive(Component)]
#[shaku(interface = SOTAActivationRepositry)]
pub struct SOTAActivationRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl SOTAActivationRepositry for SOTAActivationRepositryImpl {
    async fn update_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()> {
        eprintln!("Update SOTA alerts for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<SOTAAlert>> {
        eprintln!("Find SOTA alerts with {:?} ", event);
        todo!()
    }
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete SOTA alerts with {:?} ", event);
        Ok(())
    }
    async fn update_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()> {
        eprintln!("Update SOTA spots for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<SOTASpot>> {
        eprintln!("Find SOTA spots with {:?} ", event);
        todo!()
    }
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete SOTA spots with {:?} ", event);
        Ok(())
    }
}
