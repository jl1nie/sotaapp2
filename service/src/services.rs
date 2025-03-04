use async_trait::async_trait;
use domain::model::aprslog::AprsLog;
use shaku::Interface;
use std::collections::HashMap;

use aprs_message::AprsData;

use common::error::AppResult;
use domain::model::activation::{Alert, Spot};
use domain::model::event::{
    DeleteRef, FindAct, FindAprs, FindLog, FindRef, FindResult, GroupBy, PagenatedResult,
};
use domain::model::geomag::GeomagIndex;
use domain::model::id::{LogId, UserId};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{ParkCode, PotaReference};
use domain::model::sota::{SotaReference, SummitCode};

use crate::model::locator::UploadMuniCSV;
use crate::model::pota::{UploadPOTALog, UploadPOTAReference};
use crate::model::sota::{UploadSOTALog, UploadSOTASummit, UploadSOTASummitOpt};

#[async_trait]
pub trait UserService: Send + Sync + Interface {
    async fn find_references(&self, event: FindRef) -> AppResult<FindResult>;

    async fn find_alerts(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Alert>>>;
    async fn find_spots(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Spot>>>;

    async fn upload_activator_csv(&self, lod_id: LogId, event: UploadActivatorCSV)
        -> AppResult<()>;
    async fn upload_hunter_csv(&self, log_id: LogId, event: UploadHunterCSV) -> AppResult<()>;

    async fn upload_sota_csv(&self, user_id: UserId, event: UploadSOTALog) -> AppResult<()>;
    async fn delete_sota_log(&self, user_id: UserId) -> AppResult<()>;

    async fn award_progress(&self, user_id: UserId, query: FindLog) -> AppResult<String>;

    async fn find_century_code(&self, muni_code: i32) -> AppResult<MunicipalityCenturyCode>;
    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String>;
    async fn find_aprslog(&self, event: FindAprs) -> AppResult<Vec<AprsLog>>;
    async fn get_geomagnetic(&self) -> AppResult<Option<GeomagIndex>>;
}

#[async_trait]
pub trait AdminService: Send + Sync + Interface {
    async fn import_summit_list(&self, event: UploadSOTASummit) -> AppResult<()>;
    async fn update_summit_list(&self, event: UploadSOTASummit) -> AppResult<()>;
    async fn import_summit_opt_list(&self, event: UploadSOTASummitOpt) -> AppResult<()>;
    async fn import_pota_park_list(&self, event: UploadPOTAReference) -> AppResult<()>;
    async fn import_muni_century_list(&self, event: UploadMuniCSV) -> AppResult<()>;
    async fn show_sota_reference(&self, query: FindRef) -> AppResult<SotaReference>;
    async fn show_all_sota_references(
        &self,
        query: FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>>;

    async fn update_sota_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn delete_sota_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;
    async fn show_pota_reference(&self, query: FindRef) -> AppResult<PotaReference>;
    async fn show_all_pota_references(
        &self,
        query: FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>>;
    async fn update_pota_reference(&self, references: Vec<PotaReference>) -> AppResult<()>;
    async fn delete_pota_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()>;
    async fn health_check(&self) -> AppResult<bool>;
}

#[async_trait]
pub trait AdminPeriodicService: Send + Sync + Interface {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()>;
    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()>;
    async fn aprs_packet_received(&self, packet: AprsData) -> AppResult<()>;
}
