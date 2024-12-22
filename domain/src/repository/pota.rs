use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};
use crate::model::pota::{POTAReference, ParkCode};

#[async_trait]
pub trait POTAReferenceRepositry: Send + Sync + Interface {
    async fn create_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>>;
    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()>;
}
