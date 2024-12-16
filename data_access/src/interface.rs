use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use domain::model::pota::{POTAAlert, POTAReference, POTASpot, ParkCode};
use domain::model::sota::{SOTAAlert, SOTARefOptInfo, SOTAReference, SOTASpot, SummitCode};

use domain::model::common::event::{
    CreateRef, DeleteAct, DeleteRef, FindAct, FindRef, FindResult, UpdateAct, UpdateRef,
};

#[async_trait]
pub trait SOTADatabase: Send + Sync + Interface {
    async fn import_reference(&self, event: CreateRef<SOTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<SOTAReference>>;
    async fn update_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()>;
    async fn delete_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()>;
}

#[async_trait]
pub trait SOTAActivationDatabase: Send + Sync + Interface {
    async fn update_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()>;
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<SOTAAlert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()>;
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<SOTASpot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}

#[async_trait]
pub trait POTADatabase: Send + Sync + Interface {
    async fn import_reference(&self, event: CreateRef<POTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>>;
    async fn update_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()>;
}

#[async_trait]
pub trait POTActivationDatabase: Send + Sync + Interface {
    async fn update_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()>;
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<POTAAlert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()>;
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<POTASpot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}

#[async_trait]
pub trait HealthCheck: Send + Sync + Interface {
    async fn check_database(&self) -> AppResult<bool>;
}
