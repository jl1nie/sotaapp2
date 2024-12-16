use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::pota::{POTAAlert, POTASpot};

use domain::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};

use crate::database::ConnectionPool;
use domain::repository::pota::POTActivationDatabase;

#[derive(Component)]
#[shaku(interface = POTActivationDatabase)]
pub struct POTActivationDatabaseImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl POTActivationDatabase for POTActivationDatabaseImpl {
    async fn update_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()> {
        todo!()
    }
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<POTAAlert>> {
        todo!()
    }
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()> {
        todo!()
    }
    async fn update_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()> {
        todo!()
    }
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<POTASpot>> {
        todo!()
    }
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()> {
        todo!()
    }
}
