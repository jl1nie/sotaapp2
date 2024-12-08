use async_trait::async_trait;

use crate::interface::AdminApp;
use crate::model::pota::{event::UploadPOTACSV, POTAReference, ParkCode};
use crate::model::sota::{
    event::{UploadSOTACSV, UploadSOTAOptCSV},
    SOTARefOptInfo, SummitCode,
};

use crate::model::common::event::{DeleteRef, FindRef, FindResult, UpdateRef};

use common::error::AppResult;

pub struct AdminAppImpl {}

#[async_trait]
impl AdminApp for AdminAppImpl {
    async fn import_summit_list(&self, event: UploadSOTACSV) -> AppResult<()> {
        todo!()
    }

    async fn import_summit_opt_list(&self, event: UploadSOTAOptCSV) -> AppResult<()> {
        todo!()
    }

    async fn import_pota_park_list(&self, event: UploadPOTACSV) -> AppResult<()> {
        todo!()
    }

    async fn find_sota_reference_opt(
        &self,
        event: FindRef,
    ) -> AppResult<FindResult<SOTARefOptInfo>> {
        todo!()
    }

    async fn update_sota_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()> {
        todo!()
    }

    async fn delete_sota_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        todo!()
    }

    async fn find_pota_reference(&self, event: FindRef) -> AppResult<FindResult<POTAReference>> {
        todo!()
    }

    async fn update_pota_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()> {
        todo!()
    }

    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        todo!()
    }

    async fn health_check(&self) -> AppResult<bool> {
        todo!()
    }
}
