use async_trait::async_trait;
use chrono::{Local, NaiveDate};

use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::config::AppConfig;
use common::csv_reader::csv_reader;
use common::error::AppResult;

use domain::model::common::event::{DeleteRef, FindRef, FindRefBuilder, PagenatedResult};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{POTAReference, ParkCode};
use domain::model::sota::{SOTAReference, SummitCode};
use domain::repository::{
    healthcheck::HealthCheckRepositry, locator::LocatorRepositry, pota::POTAReferenceRepositry,
    sota::SOTAReferenceReposity,
};

use crate::model::locator::{MuniCSVFile, UploadMuniCSV};
use crate::model::pota::{POTACSVFile, UploadPOTACSV};
use crate::model::sota::{SOTACSVFile, SOTACSVOptFile};
use crate::model::sota::{UploadSOTACSV, UploadSOTAOptCSV};

use crate::services::AdminService;

#[derive(Component)]
#[shaku(interface = AdminService)]
pub struct AdminServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SOTAReferenceReposity>,
    #[shaku(inject)]
    pota_repo: Arc<dyn POTAReferenceRepositry>,
    #[shaku(inject)]
    check_repo: Arc<dyn HealthCheckRepositry>,
    #[shaku(inject)]
    loc_repo: Arc<dyn LocatorRepositry>,
    config: AppConfig,
}

#[async_trait]
impl AdminService for AdminServiceImpl {
    async fn import_summit_list(&self, UploadSOTACSV { data }: UploadSOTACSV) -> AppResult<()> {
        let today = Local::now().date_naive();
        let csv: Vec<SOTACSVFile> = csv_reader(data, 2)?;

        let is_valid_summit = |r: &SOTAReference| -> bool {
            let validfrom = NaiveDate::parse_from_str(&r.valid_from, "%d/%m/%Y").unwrap_or(today);
            let validto = NaiveDate::parse_from_str(&r.valid_to, "%d/%m/%Y").unwrap_or(today);
            self.config
                .sota_import_association
                .as_ref()
                .map_or(true, |re| re.is_match(&r.summit_code))
                && today <= validto
                && today >= validfrom
        };
        let req: Vec<_> = csv
            .into_iter()
            .map(SOTAReference::from)
            .filter(is_valid_summit)
            .collect();
        tracing::info!("import {} references.", req.len());
        self.sota_repo
            .delete_reference(DeleteRef::DeleteAll)
            .await?;
        self.sota_repo.create_reference(req).await?;
        Ok(())
    }

    async fn import_summit_opt_list(
        &self,
        UploadSOTAOptCSV { data }: UploadSOTAOptCSV,
    ) -> AppResult<()> {
        let csv: Vec<SOTACSVOptFile> = csv_reader(data, 1)?;

        let ja_hash: HashMap<_, _> = csv
            .into_iter()
            .map(|r| (r.summit_code.clone(), r))
            .collect();

        let associations: HashSet<String> = ja_hash
            .keys()
            .cloned()
            .map(|s| s.split("/").next().unwrap_or("").to_owned() + "/")
            .collect();

        for assoc in associations {
            let query = FindRefBuilder::new().sota().name(assoc).build();
            let result = self.sota_repo.find_reference(&query).await?;
            let newref = result
                .into_iter()
                .filter(|r| ja_hash.contains_key(&r.summit_code))
                .map(|mut r| {
                    let ja = ja_hash.get(&r.summit_code).unwrap();
                    r.summit_name = ja.summit_name.clone();
                    r.summit_name_j = Some(ja.summit_name_j.clone());
                    r.city = Some(ja.city.clone());
                    r.city_j = Some(ja.city_j.clone());
                    r.longitude = ja.longitude;
                    r.latitude = ja.latitude;
                    r.alt_m = ja.alt_m;
                    r
                })
                .collect();
            self.sota_repo.update_reference(newref).await?;
        }
        Ok(())
    }

    async fn import_pota_park_list(&self, UploadPOTACSV { data }: UploadPOTACSV) -> AppResult<()> {
        let requests: Vec<POTACSVFile> = csv_reader(data, 1)?;
        let newref = requests.into_iter().map(POTAReference::from).collect();
        self.pota_repo
            .delete_reference(DeleteRef::DeleteAll)
            .await?;
        self.pota_repo.create_reference(newref).await?;
        Ok(())
    }

    async fn import_muni_century_list(
        &self,
        UploadMuniCSV { data }: UploadMuniCSV,
    ) -> AppResult<()> {
        let requests: Vec<MuniCSVFile> = csv_reader(data, 1)?;
        let newtable = requests
            .into_iter()
            .map(MunicipalityCenturyCode::from)
            .collect();
        self.loc_repo.upload_muni_century_list(newtable).await?;
        Ok(())
    }

    async fn show_sota_reference(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<SOTAReference>> {
        Ok(self.sota_repo.show_reference(&event).await?)
    }

    async fn update_sota_reference(&self, references: Vec<SOTAReference>) -> AppResult<()> {
        self.sota_repo.update_reference(references).await?;
        Ok(())
    }

    async fn delete_sota_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        self.sota_repo.delete_reference(event).await?;
        Ok(())
    }

    async fn show_pota_reference(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<POTAReference>> {
        Ok(self.pota_repo.show_reference(&event).await?)
    }

    async fn update_pota_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        self.pota_repo.update_reference(references).await?;
        Ok(())
    }

    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        self.pota_repo.delete_reference(event).await?;
        Ok(())
    }
    async fn health_check(&self) -> AppResult<bool> {
        Ok(self.check_repo.check_database().await?)
    }
}
