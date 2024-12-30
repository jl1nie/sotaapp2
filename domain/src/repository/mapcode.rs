use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::FindRef;

#[async_trait]
pub trait MapCodeRepositry: Send + Sync + Interface {
    async fn find_mapcode(&self, query: &FindRef) -> AppResult<String>;
}
