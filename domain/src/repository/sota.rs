use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::event::{DeleteRef, FindRef, PagenatedResult};
use crate::model::sota::{SOTAReference, SummitCode};

#[async_trait]
pub trait SOTARepository: Send + Sync + Interface {
    async fn find_reference(&self, query: &FindRef) -> AppResult<Vec<SOTAReference>>;

    async fn create_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn show_reference(&self, query: &FindRef) -> AppResult<SOTAReference>;
    async fn show_all_references(
        &self,
        query: &FindRef,
    ) -> AppResult<PagenatedResult<SOTAReference>>;
    async fn update_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn upsert_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;
}
