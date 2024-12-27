use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::{DeleteRef, FindRef, FindResult};
use crate::model::sota::{SOTAReference, SummitCode};

#[async_trait]
pub trait SOTAReferenceReposity: Send + Sync + Interface {
    async fn create_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn find_reference(&self, query: &FindRef) -> AppResult<FindResult<SOTAReference>>;
    async fn update_reference(&self, references: Vec<SOTAReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;
}
