use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::pota::{POTAAlert, POTASpot};

use domain::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};

use crate::database::ConnectionPool;
use domain::repository::pota::POTAActivationRepositry;

#[derive(Component)]
#[shaku(interface = POTAActivationRepositry)]
pub struct POTActivationRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl POTAActivationRepositry for POTActivationRepositryImpl {
    async fn update_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()> {
        eprintln!("Update POTA alerts for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<POTAAlert>> {
        eprintln!("Find POTA alerts with {:?} ", event);
        todo!()
    }
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete POTA alerts with {:?} ", event);
        Ok(())
    }
    async fn update_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()> {
        eprintln!("Update POTA spots for {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<POTASpot>> {
        eprintln!("Find POTA spots with {:?} ", event);
        todo!()
    }
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()> {
        eprintln!("Delete POTA spots with {:?} ", event);
        Ok(())
    }
}
