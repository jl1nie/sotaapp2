use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::{DeleteLog, DeleteRef, FindRef};
use crate::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference, ParkCode};

#[async_trait]
pub trait POTAReferenceRepositry: Send + Sync + Interface {
    async fn create_reference(&self, refernces: Vec<POTAReference>) -> AppResult<()>;
    async fn find_reference(&self, query: &FindRef) -> AppResult<Vec<POTAReference>>;
    async fn update_reference(&self, refernces: Vec<POTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()>;
    async fn upload_activator_log(&self, logs: Vec<POTAActivatorLog>) -> AppResult<()>;
    async fn upload_hunter_log(&self, logs: Vec<POTAHunterLog>) -> AppResult<()>;
    async fn delete_log(&self, query: DeleteLog) -> AppResult<()>;
}
