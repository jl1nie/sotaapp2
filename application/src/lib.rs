use async_trait::async_trait;
use geo_types::Rect;

pub mod model;

use crate::model::sota::{
    event::{CreateRef, CreateRefs, DeleteRef, UpdateRef, UpdateRefs},
    SOTAReference,
};
use common::error::AppResult;

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check_db(&self) -> bool;
}

#[async_trait]
pub trait SOTA: Send + Sync {
    async fn create_a_reference(&self, event: CreateRef) -> AppResult<()>;
    async fn create_references(&self, event: CreateRefs) -> AppResult<()>;
    async fn update_a_reference(&self, event: UpdateRef) -> AppResult<()>;
    async fn update_references(&self, event: UpdateRefs) -> AppResult<()>;
    async fn delete_a_reference(&self, event: DeleteRef) -> AppResult<()>;
    async fn find_by_summit_code(&self, summit_code: &str) -> AppResult<Option<SOTAReference>>;
    async fn find_by_location(&self, location: &Rect) -> AppResult<Vec<SOTAReference>>;
}
