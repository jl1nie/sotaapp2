use anyhow::Result;
use async_trait::async_trait;
use geo_types::Rect;

pub mod model;

use crate::model::sota::{event::CreateARef, event::CreateRefs, SOTAReference};
use common::error::AppResult;

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check_db(&self) -> bool;
}

#[async_trait]
pub trait SOTA: Send + Sync {
    async fn create_a_reference(&self, event: CreateARef) -> AppResult<()>;
    async fn create_references(&self, event: CreateRefs) -> AppResult<()>;
    async fn find_by_summit_id(&self, summit_id: &str) -> AppResult<Option<SOTAReference>>;
    async fn find_by_location(&self, location: &Rect) -> AppResult<Vec<SOTAReference>>;
}
