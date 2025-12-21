use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::locator::MunicipalityCenturyCode;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait LocatorRepositry: Send + Sync + Interface {
    async fn upload_muni_century_list(&self, table: Vec<MunicipalityCenturyCode>) -> AppResult<()>;
    async fn find_location_by_muni_code(
        &self,
        muni_code: i32,
    ) -> AppResult<MunicipalityCenturyCode>;
    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String>;
}
