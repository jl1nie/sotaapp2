use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::pota::{POTAReference, ParkCode};

use domain::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};

use crate::database::ConnectionPool;
use domain::repository::pota::POTAReferenceRepositry;

#[derive(Component)]
#[shaku(interface = POTAReferenceRepositry)]
pub struct POTAReferenceRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

#[async_trait]
impl POTAReferenceRepositry for POTAReferenceRepositryImpl {
    async fn create_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()> {
        eprintln!("Create POTA {} refrences.", event.requests.len());
        Ok(())
    }
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>> {
        eprintln!("Find POTA references with {:?}.", event);
        todo!()
    }

    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()> {
        eprintln!("Update POTA {} refrences.", event.requests.len());
        Ok(())
    }
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        eprintln!("Delete POTA {:?}", event);
        Ok(())
    }
}
