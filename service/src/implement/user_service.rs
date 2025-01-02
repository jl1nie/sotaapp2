use async_trait::async_trait;
use chrono::Utc;
use common::config::AppConfig;
use common::csv_reader::csv_reader;
use common::error::AppResult;
use domain::model::common::id::UserId;
use shaku::Component;
use std::sync::Arc;

use crate::model::pota::{
    POTAActivatorLogCSV, POTAHunterLogCSV, UploadActivatorCSV, UploadHunterCSV,
};
use crate::services::UserService;

use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{DeleteLog, FindAct, FindAppResult, FindRef};
use domain::model::locator::MunicipalityCenturyCode;

use domain::repository::activation::ActivationRepositry;
use domain::repository::locator::LocatorRepositry;
use domain::repository::pota::POTAReferenceRepositry;
use domain::repository::sota::SOTAReferenceReposity;

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SOTAReferenceReposity>,
    #[shaku(inject)]
    pota_repo: Arc<dyn POTAReferenceRepositry>,
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    locator_repo: Arc<dyn LocatorRepositry>,
    config: AppConfig,
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn find_references(&self, event: FindRef) -> AppResult<FindAppResult> {
        let mut result = FindAppResult::default();

        if event.is_sota() {
            result.sota(self.sota_repo.find_reference(&event).await?)
        }
        if event.is_pota() {
            result.pota(self.pota_repo.find_reference(&event).await?)
        }
        Ok(result)
    }

    async fn find_alerts(&self, event: FindAct) -> AppResult<Vec<Alert>> {
        Ok(self.act_repo.find_alerts(&event).await?)
    }

    async fn find_spots(&self, event: FindAct) -> AppResult<Vec<Spot>> {
        Ok(self.act_repo.find_spots(&event).await?)
    }

    async fn upload_activator_csv(
        &self,
        user_id: UserId,
        UploadActivatorCSV { data }: UploadActivatorCSV,
    ) -> AppResult<()> {
        let requests: Vec<POTAActivatorLogCSV> = csv_reader(data, 1)?;
        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| POTAActivatorLogCSV::to_log(user_id, l))
            .collect();
        self.pota_repo.upload_activator_log(newlog).await?;
        self.pota_repo
            .delete_log(DeleteLog {
                before: Utc::now() - self.config.log_expire,
            })
            .await?;
        Ok(())
    }

    async fn upload_hunter_csv(
        &self,
        user_id: UserId,
        UploadHunterCSV { data }: UploadHunterCSV,
    ) -> AppResult<()> {
        let requests: Vec<POTAHunterLogCSV> = csv_reader(data, 1)?;
        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| POTAHunterLogCSV::to_log(user_id, l))
            .collect();
        self.pota_repo.upload_hunter_log(newlog).await?;
        self.pota_repo
            .delete_log(DeleteLog {
                before: Utc::now() - self.config.log_expire,
            })
            .await?;
        Ok(())
    }

    async fn find_century_code(&self, muni_code: i32) -> AppResult<MunicipalityCenturyCode> {
        let result = self
            .locator_repo
            .find_location_by_muni_code(muni_code)
            .await?;
        Ok(result)
    }

    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String> {
        Ok(self.locator_repo.find_mapcode(lon, lat).await?)
    }
}
