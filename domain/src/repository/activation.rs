use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::activation::{Alert, Spot};
use crate::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};

#[async_trait]
pub trait ActivationRepositry: Send + Sync + Interface {
    async fn update_alert(&self, event: UpdateAct<Alert>) -> AppResult<()>;
    async fn find_alert(&self, event: &FindAct) -> AppResult<FindResult<Alert>>;
    async fn delete_alert(&self, event: DeleteAct) -> AppResult<()>;
    async fn update_spot(&self, event: UpdateAct<Spot>) -> AppResult<()>;
    async fn find_spot(&self, event: &FindAct) -> AppResult<FindResult<Spot>>;
    async fn delete_spot(&self, event: DeleteAct) -> AppResult<()>;
}
