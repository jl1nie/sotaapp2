use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::event::{DeleteLog, DeleteRef, FindLog, FindRef, PagenatedResult};
use crate::model::sota::{SotaLog, SotaReference, SummitCode};

#[async_trait]
pub trait SotaRepository: Send + Sync + Interface {
    async fn find_reference(&self, query: &FindRef) -> AppResult<Vec<SotaReference>>;

    async fn create_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn show_reference(&self, query: &FindRef) -> AppResult<SotaReference>;
    async fn show_all_references(
        &self,
        query: &FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>>;
    async fn update_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn upsert_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;

    async fn upload_log(&self, logs: Vec<SotaLog>) -> AppResult<()>;
    async fn find_log(&self, query: &FindLog) -> AppResult<Vec<SotaLog>>;
    async fn delete_log(&self, query: DeleteLog) -> AppResult<()>;
}
