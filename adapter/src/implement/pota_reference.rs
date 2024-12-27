use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::AppResult;

use domain::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference, ParkCode};

use domain::model::common::event::{DeleteLog, DeleteRef, FindRef, FindResult};

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
    async fn create_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        tracing::info!("Create POTA {} refrences.", references.len());
        Ok(())
    }
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>> {
        tracing::info!("Find POTA references with {:?}.", event);
        todo!()
    }

    async fn update_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        tracing::info!("Update POTA {} refrences.", references.len());
        Ok(())
    }
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        tracing::info!("Delete POTA {:?}", event);
        Ok(())
    }

    async fn upload_activator_log(&self, logs: Vec<POTAActivatorLog>) -> AppResult<()> {
        tracing::info!("Upload POTA activator log.");
        Ok(())
    }

    async fn upload_hunter_log(&self, logs: Vec<POTAHunterLog>) -> AppResult<()> {
        tracing::info!("Upload POTA hunter log.");
        Ok(())
    }

    async fn delete_activator_log(&self, query: DeleteLog) -> AppResult<()> {
        tracing::info!("Delete Activator log.");
        Ok(())
    }

    async fn delete_hunter_log(&self, query: DeleteLog) -> AppResult<()> {
        tracing::info!("Delete Hunter log.");
        Ok(())
    }
}
