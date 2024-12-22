use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};
use crate::model::sota::{SOTAReference, SummitCode};

#[async_trait]
pub trait SOTAReferenceReposity: Send + Sync + Interface {
    async fn create_reference(&self, event: CreateRef<SOTAReference>) -> AppResult<()>;
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<SOTAReference>>;
    async fn update_reference(&self, event: UpdateRef<SOTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()>;
}
