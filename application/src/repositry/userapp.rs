use async_trait::async_trait;

use crate::model::pota::{event::UploadPOTACSV, POTAAlert, POTAReference, POTASpot, ParkCode};
use crate::model::sota::{
    event::{UploadSOTACSV, UploadSOTAOptCSV},
    SOTAAlert, SOTARefOptInfo, SOTAReference, SOTASpot, SummitCode,
};

use crate::model::common::event::{
    DeleteRef, FindAct, FindAppResult, FindRef, FindResult, UpdateAct, UpdateRef,
};

use common::error::AppResult;

use crate::interface::UserApp;

pub struct UserAppImpl {}

#[async_trait]
impl UserApp for UserAppImpl {
    async fn find_reference(
        &self,
        event: FindRef,
    ) -> AppResult<FindAppResult<SOTAReference, POTAReference>> {
        todo!()
    }

    async fn find_alert(&self, event: FindAct) -> AppResult<FindAppResult<SOTAAlert, POTAAlert>> {
        todo!()
    }

    async fn find_spot(&self, event: FindAct) -> AppResult<FindAppResult<SOTASpot, POTASpot>> {
        todo!()
    }
}
