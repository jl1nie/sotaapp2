use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::activation::{Alert, Spot};
use crate::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};

#[async_trait]
pub trait ActivationRepositry: Send + Sync + Interface {
    async fn update_alerts(&self, event: UpdateAct<Alert>) -> AppResult<()>;
    async fn find_alerts(&self, event: &FindAct) -> AppResult<FindResult<Alert>>;
    async fn delete_alerts(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spots(&self, event: UpdateAct<Spot>) -> AppResult<()>;
    async fn find_spots(&self, event: &FindAct) -> AppResult<FindResult<Spot>>;
    async fn delete_spots(&self, event: DeleteAct) -> AppResult<()>;
}
