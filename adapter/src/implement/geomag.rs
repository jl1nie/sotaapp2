use async_trait::async_trait;
use shaku::Component;
use std::sync::{Arc, Mutex};

use common::error::AppResult;
use domain::model::geomag::GeomagIndex;
use domain::repository::geomag::GeoMagRepositry;

#[derive(Component)]
#[shaku(interface = GeoMagRepositry)]
pub struct GeoMagRepositryImpl {
    latest_data: Arc<Mutex<Option<GeomagIndex>>>,
}

#[async_trait]
impl GeoMagRepositry for GeoMagRepositryImpl {
    async fn update_geomag(&self, index: GeomagIndex) -> AppResult<()> {
        let mut latest_data = self.latest_data.lock().unwrap();
        tracing::info!("Updated geomag index: {:?}", index);
        *latest_data = Some(index);
        Ok(())
    }

    async fn get_geomag(&self) -> AppResult<Option<GeomagIndex>> {
        let latest_data = self.latest_data.lock().unwrap();
        Ok(latest_data.clone())
    }
}
