use async_trait::async_trait;

use application::model::common::event::{
    CreateRef, DeleteAct, DeleteRef, FindAct, FindRef, FindResult, UpdateAct, UpdateRef,
};
use application::model::common::{Alert, Spot};
use application::model::pota::{POTAReference, ParkCode};
use application::model::sota::{SOTARefOptInfo, SOTAReference, SummitCode};
use common::error::AppResult;

#[async_trait]
pub trait SOTADatabase: Send + Sync {
    async fn create_reference(&self, event: CreateRef<SOTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: FindRef) -> AppResult<FindResult<SOTAReference>>;
    async fn update_reference(&self, event: UpdateRef<SOTAReference>) -> AppResult<()>;
    async fn update_reference_opt(&self, event: CreateRef<SOTARefOptInfo>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()>;
}

#[async_trait]
pub trait SOTAActivationDatabase: Send + Sync {
    async fn update_alert(&self, event: UpdateAct<Alert>) -> AppResult<()>;
    async fn find_alert(&self, event: FindAct) -> AppResult<FindResult<Alert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<Spot>) -> AppResult<()>;
    async fn find_spot(&self, event: FindAct) -> AppResult<FindResult<Spot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}

#[async_trait]
pub trait POTADatabase: Send + Sync {
    async fn create_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: FindRef) -> AppResult<FindResult<POTAReference>>;
    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()>;
}

#[async_trait]
pub trait POTActivationDatabase: Send + Sync {
    async fn update_alert(&self, event: UpdateAct<Alert>) -> AppResult<()>;
    async fn find_alert(&self, event: FindAct) -> AppResult<FindResult<Alert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<Spot>) -> AppResult<()>;
    async fn find_spot(&self, event: FindAct) -> AppResult<FindResult<Spot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}
