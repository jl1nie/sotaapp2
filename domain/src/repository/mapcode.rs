use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::event::FindRef;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait MapCodeRepositry: Send + Sync + Interface {
    async fn find_mapcode(&self, query: &FindRef) -> AppResult<String>;
}
