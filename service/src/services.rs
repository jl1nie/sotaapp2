use async_trait::async_trait;
use shaku::Interface;
use std::collections::HashMap;

use aprs_message::AprsData;

use crate::model::award::{AwardResult, JudgmentMode};
use crate::model::locator::UploadMuniCSV;
use crate::model::pota::{UploadPOTALog, UploadPOTAReference};
use crate::model::sota::{UploadSOTALog, UploadSOTASummit, UploadSOTASummitOpt};
use common::error::AppResult;
use domain::model::activation::{Alert, Spot, SpotLog};
use domain::model::aprslog::{AprsLog, AprsTrack};
use domain::model::event::{
    DeleteRef, FindAct, FindAprs, FindLog, FindRef, FindResult, GroupBy, PagenatedResult,
};
use domain::model::geomag::GeomagIndex;
use domain::model::id::{LogId, UserId};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{ParkCode, PotaLogHist, PotaReference};
use domain::model::sota::{SotaReference, SummitCode};
use std::path::Path;

#[async_trait]
pub trait UserService: Send + Sync + Interface {
    async fn count_references(&self, event: &FindRef) -> AppResult<i64>;
    async fn find_references(&self, event: FindRef) -> AppResult<FindResult>;

    async fn find_alerts(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Alert>>>;
    async fn find_spots(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<SpotLog>>>;

    async fn upload_pota_log(&self, event: UploadPOTALog) -> AppResult<PotaLogHist>;
    async fn delete_pota_log(&self, log_id: LogId) -> AppResult<()>;
    async fn find_logid(&self, log_id: LogId) -> AppResult<PotaLogHist>;

    async fn upload_sota_log(&self, user_id: UserId, event: UploadSOTALog) -> AppResult<()>;
    async fn delete_sota_log(&self, user_id: UserId) -> AppResult<()>;

    async fn award_progress(&self, user_id: UserId, query: FindLog) -> AppResult<String>;

    /// SOTA日本支部設立10周年記念アワード判定（in-memory、DBに保存しない）
    fn judge_10th_anniversary_award(
        &self,
        csv_data: &str,
        mode: JudgmentMode,
    ) -> AppResult<AwardResult>;

    async fn find_century_code(&self, muni_code: i32) -> AppResult<MunicipalityCenturyCode>;
    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String>;
    async fn find_aprs_log(&self, event: FindAprs) -> AppResult<Vec<AprsLog>>;
    async fn get_aprs_track(&self, event: FindAprs) -> AppResult<Vec<AprsTrack>>;
    async fn get_geomagnetic(&self) -> AppResult<Option<GeomagIndex>>;
}

#[async_trait]
pub trait AdminService: Send + Sync + Interface {
    async fn import_summit_list(&self, event: UploadSOTASummit) -> AppResult<usize>;
    async fn update_summit_list(&self, event: UploadSOTASummit) -> AppResult<usize>;
    /// メモリ効率の良いサミットリスト更新（ファイルから2回読み込み）
    async fn update_summit_list_from_file(&self, path: &Path) -> AppResult<usize>;
    async fn import_summit_opt_list(&self, event: UploadSOTASummitOpt) -> AppResult<usize>;
    async fn import_pota_park_list(&self, event: UploadPOTAReference) -> AppResult<usize>;
    /// メモリ効率の良いパークリスト更新（ファイルから2回読み込み）
    async fn update_pota_park_list_from_file(&self, path: &Path) -> AppResult<usize>;
    async fn import_pota_park_list_ja(&self, event: UploadPOTAReference) -> AppResult<usize>;
    async fn import_muni_century_list(&self, event: UploadMuniCSV) -> AppResult<usize>;
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
