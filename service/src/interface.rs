use async_trait::async_trait;
use shaku::Interface;

use domain::model::pota::{event::UploadPOTACSV, POTAAlert, POTAReference, POTASpot, ParkCode};
use domain::model::sota::{
    event::{UploadSOTACSV, UploadSOTAOptCSV},
    SOTAAlert, SOTARefOptInfo, SOTAReference, SOTASpot, SummitCode,
};

use domain::model::common::event::{
    DeleteRef, FindAct, FindAppResult, FindRef, FindResult, UpdateAct, UpdateRef,
};

use common::error::AppResult;

#[async_trait]
pub trait UserService: Send + Sync + Interface {
    async fn find_reference(
        &self,
        event: FindRef,
    ) -> AppResult<FindAppResult<SOTAReference, POTAReference>>;
    async fn find_alert(&self, event: FindAct) -> AppResult<FindAppResult<SOTAAlert, POTAAlert>>;
    async fn find_spot(&self, event: FindAct) -> AppResult<FindAppResult<SOTASpot, POTASpot>>;
}

#[async_trait]
pub trait AdminService: Send + Sync + Interface {
    async fn import_summit_list(&self, event: UploadSOTACSV) -> AppResult<()>;
    async fn import_summit_opt_list(&self, event: UploadSOTAOptCSV) -> AppResult<()>;
    async fn import_pota_park_list(&self, event: UploadPOTACSV) -> AppResult<()>;

    async fn find_sota_reference(&self, event: FindRef) -> AppResult<FindResult<SOTAReference>>;
    async fn update_sota_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()>;
    async fn delete_sota_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()>;

    async fn find_pota_reference(&self, event: FindRef) -> AppResult<FindResult<POTAReference>>;
    async fn update_pota_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()>;
    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()>;

    async fn health_check(&self) -> AppResult<bool>;
}

#[async_trait]
pub trait AdminPeriodicService: Send + Sync + Interface {
    async fn update_sota_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()>;
    async fn update_sota_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()>;
    async fn update_pota_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()>;
    async fn update_pota_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()>;
}
