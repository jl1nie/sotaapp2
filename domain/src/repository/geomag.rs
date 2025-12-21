use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::geomag::GeomagIndex;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait GeoMagRepositry: Send + Sync + Interface {
    async fn get_geomag(&self) -> AppResult<Option<GeomagIndex>>;
}
