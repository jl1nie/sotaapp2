use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::common::event::FindResult;
use crate::model::locator::MunicipalityCenturyCode;

#[async_trait]
pub trait LocatorRepositry: Send + Sync + Interface {
    async fn upload_muni_century_list(&self, table: Vec<MunicipalityCenturyCode>) -> AppResult<()>;
    async fn find_location_by_muni_code(
        &self,
        muni_code: i32,
    ) -> AppResult<FindResult<MunicipalityCenturyCode>>;
    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String>;
}
