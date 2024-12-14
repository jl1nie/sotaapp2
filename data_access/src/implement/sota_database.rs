use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;
use domain::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};
use domain::model::sota::{SOTARefOptInfo, SOTAReference, SummitCode};

use crate::database::ConnectionPool;
use crate::interface::SOTADatabase;

#[derive(Component)]
#[shaku(interface = SOTADatabase)]
pub struct SOTADatabaseImpl {
    config: AppConfig,
    pool: ConnectionPool,
}
#[async_trait]
impl SOTADatabase for SOTADatabaseImpl {
    async fn create_reference(&self, event: CreateRef<SOTAReference>) -> AppResult<()> {
        todo!()
    }
    async fn create_reference_opt(&self, event: CreateRef<SOTARefOptInfo>) -> AppResult<()> {
        todo!()
    }
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<SOTAReference>> {
        todo!()
    }
    async fn update_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()> {
        todo!()
    }
    async fn delete_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        todo!()
    }
}
