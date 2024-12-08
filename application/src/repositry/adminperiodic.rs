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

use crate::interface::AdminPeriodic;
pub struct AdminPeriodicImpl {}

#[async_trait]
impl AdminPeriodic for AdminPeriodicImpl {
    async fn update_sota_alert(&self, event: UpdateAct<SOTAAlert>) -> AppResult<()> {
        todo!()
    }

    async fn update_sota_spot(&self, event: UpdateAct<SOTASpot>) -> AppResult<()> {
        todo!()
    }

    async fn update_pota_alert(&self, event: UpdateAct<POTAAlert>) -> AppResult<()> {
        todo!()
    }
    async fn update_pota_spot(&self, event: UpdateAct<POTASpot>) -> AppResult<()> {
        todo!()
    }
}
