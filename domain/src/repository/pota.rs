use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::event::{DeleteLog, DeleteRef, FindRef, PagenatedResult};
use crate::model::id::LogId;
use crate::model::pota::{
    ParkCode, PotaActLog, PotaHuntLog, PotaLogHist, PotaLogStat, PotaRefLog, PotaReference,
};

#[async_trait]
pub trait PotaRepository: Send + Sync + Interface {
    async fn find_reference(&self, query: &FindRef) -> AppResult<Vec<PotaRefLog>>;

    async fn create_reference(&self, refernces: Vec<PotaReference>) -> AppResult<()>;
    async fn show_reference(&self, query: &FindRef) -> AppResult<PotaReference>;
    async fn show_all_references(
        &self,
        query: &FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>>;
    async fn update_reference(&self, refernces: Vec<PotaReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()>;

    async fn upload_activator_log(&self, logs: Vec<PotaActLog>) -> AppResult<()>;
    async fn upload_hunter_log(&self, logs: Vec<PotaHuntLog>) -> AppResult<()>;
    async fn delete_log(&self, query: DeleteLog) -> AppResult<()>;
    async fn log_statistics(&self) -> AppResult<PotaLogStat>;
    async fn find_logid(&self, query: LogId) -> AppResult<PotaLogHist>;
    async fn update_logid(&self, log: PotaLogHist) -> AppResult<()>;
    async fn migrate_legacy_log(&self, dbname: String) -> AppResult<()>;
}
