use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::{
    CreateRef, DeleteAct, DeleteRef, FindAct, FindRef, FindResult, UpdateAct, UpdateRef,
};
use crate::model::pota::{POTAAlert, POTAReference, POTASpot, ParkCode};

#[async_trait]
pub trait POTAReferenceRepositry: Send + Sync + Interface {
    async fn import_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>>;
    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()>;
}

#[async_trait]
pub trait POTAActivationRepositry: Send + Sync + Interface {
    async fn update_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()>;
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<POTAAlert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()>;
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<POTASpot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}
