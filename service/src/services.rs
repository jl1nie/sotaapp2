use async_trait::async_trait;
use shaku::Interface;

use common::error::AppResult;
use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{DeleteRef, FindAct, FindRef, FindResult, PagenatedResult};
use domain::model::common::id::UserId;
use domain::model::geomag::GeomagIndex;
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{POTAReference, ParkCode};
use domain::model::sota::{SOTAReference, SummitCode};

use crate::model::locator::UploadMuniCSV;
use crate::model::pota::{UploadActivatorCSV, UploadHunterCSV, UploadPOTACSV};
use crate::model::sota::{UploadSOTACSV, UploadSOTAOptCSV};

#[async_trait]
pub trait UserService: Send + Sync + Interface {
    async fn find_references(&self, event: FindRef) -> AppResult<FindResult>;
    async fn find_alerts(&self, event: FindAct) -> AppResult<Vec<Alert>>;
    async fn find_spots(&self, event: FindAct) -> AppResult<Vec<Spot>>;
    async fn upload_activator_csv(
        &self,
        user_id: UserId,
        event: UploadActivatorCSV,
    ) -> AppResult<()>;
    async fn upload_hunter_csv(&self, user_id: UserId, event: UploadHunterCSV) -> AppResult<()>;

    async fn find_century_code(&self, muni_code: i32) -> AppResult<MunicipalityCenturyCode>;
    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String>;

    async fn get_geomagnetic(&self) -> AppResult<Option<GeomagIndex>>;
}

#[async_trait]
pub trait AdminService: Send + Sync + Interface {
    async fn import_summit_list(&self, event: UploadSOTACSV) -> AppResult<()>;
    async fn update_summit_list(&self, event: UploadSOTACSV) -> AppResult<()>;
    async fn import_summit_opt_list(&self, event: UploadSOTAOptCSV) -> AppResult<()>;
    async fn import_pota_park_list(&self, event: UploadPOTACSV) -> AppResult<()>;
    async fn import_muni_century_list(&self, event: UploadMuniCSV) -> AppResult<()>;
    async fn show_sota_reference(&self, query: FindRef) -> AppResult<SOTAReference>;
    async fn show_all_sota_references(
        &self,
        query: FindRef,
    ) -> AppResult<PagenatedResult<SOTAReference>>;

    async fn update_sota_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn delete_sota_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;
    async fn show_pota_reference(&self, query: FindRef) -> AppResult<POTAReference>;
    async fn show_all_pota_references(
        &self,
        query: FindRef,
    ) -> AppResult<PagenatedResult<POTAReference>>;
    async fn update_pota_reference(&self, references: Vec<POTAReference>) -> AppResult<()>;
    async fn delete_pota_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()>;
    async fn health_check(&self) -> AppResult<bool>;
}

#[async_trait]
pub trait AdminPeriodicService: Send + Sync + Interface {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()>;
    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()>;
    async fn update_geomag(&self, index: GeomagIndex) -> AppResult<()>;
}
