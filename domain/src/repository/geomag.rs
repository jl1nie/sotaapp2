use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

use crate::model::geomag::GeomagIndex;
#[async_trait]
pub trait GeoMagRepositry: Send + Sync + Interface {
    async fn get_geomag(&self) -> AppResult<Option<GeomagIndex>>;
}
