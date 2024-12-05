use async_trait::async_trait;

pub mod application;
pub mod model;

use crate::model::sota::event::{
    CreateRef, CreateRefs, DeleteRef, SearchRefs, SearchResults, UpdateRef, UpdateRefs, UploadCSV,
};
use common::error::AppResult;

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check_db(&self) -> bool;
}

#[async_trait]
pub trait SOTAApplication: Send + Sync {
    async fn import_summit_list(&self, event: UploadCSV) -> AppResult<()>;
}

#[async_trait]
pub trait SOTADatabase: Send + Sync {
    async fn create_a_reference(&self, event: CreateRef) -> AppResult<()>;
    async fn create_references(&self, event: CreateRefs) -> AppResult<()>;
    async fn update_a_reference(&self, event: UpdateRef) -> AppResult<()>;
    async fn update_references(&self, event: UpdateRefs) -> AppResult<()>;
    async fn delete_a_reference(&self, event: DeleteRef) -> AppResult<()>;
    async fn search(&self, event: SearchRefs) -> AppResult<SearchResults>;
}
