use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::pota::{POTAReference, ParkCode};

use domain::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};

use crate::database::ConnectionPool;
use domain::repository::pota::POTADatabase;

#[derive(Component)]
#[shaku(interface = POTADatabase)]
pub struct POTADatabaseImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl POTADatabase for POTADatabaseImpl {
    async fn import_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()> {
        todo!()
    }
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>> {
        todo!()
    }
    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()> {
        todo!()
    }
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        todo!()
    }
}
