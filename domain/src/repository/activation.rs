use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::activation::{Alert, Spot};
use crate::model::event::{DeleteAct, FindAct};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ActivationRepositry: Send + Sync + Interface {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()>;
    async fn find_alerts(&self, query: &FindAct) -> AppResult<Vec<Alert>>;
    async fn delete_alerts(&self, query: DeleteAct) -> AppResult<()>;
    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()>;
    async fn find_spots(&self, query: &FindAct) -> AppResult<Vec<Spot>>;
    async fn delete_spots(&self, query: DeleteAct) -> AppResult<()>;
}
