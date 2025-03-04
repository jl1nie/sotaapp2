use async_trait::async_trait;
use chrono::Local;

use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::error::AppResult;
use common::utils::csv_reader;

use domain::model::event::{DeleteRef, FindRef, FindRefBuilder, PagenatedResult};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{ParkCode, PotaReference};
use domain::model::sota::{SotaReference, SummitCode};
use domain::repository::{
    healthcheck::HealthCheckRepositry, locator::LocatorRepositry, pota::PotaRepository,
    sota::SotaRepository,
};

use crate::model::locator::{MuniCSVFile, UploadMuniCSV};
use crate::model::pota::{POTACSVFile, UploadPOTACSV};
use crate::model::sota::{SOTASumitOptCSV, SOTASummitCSV};
use crate::model::sota::{UploadSOTASummit, UploadSOTASummitOpt};

use crate::services::AdminService;

#[derive(Component)]
#[shaku(interface = AdminService)]
pub struct AdminServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
    #[shaku(inject)]
    pota_repo: Arc<dyn PotaRepository>,
    #[shaku(inject)]
    check_repo: Arc<dyn HealthCheckRepositry>,
    #[shaku(inject)]
    loc_repo: Arc<dyn LocatorRepositry>,
}

fn is_valid_summit(r: &SotaReference) -> bool {
    let today = Local::now().date_naive();
    today <= r.valid_to && today >= r.valid_from
}

#[async_trait]
impl AdminService for AdminServiceImpl {
    async fn import_summit_list(
        &self,
        UploadSOTASummit { data }: UploadSOTASummit,
    ) -> AppResult<()> {
        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;
        let req: Vec<_> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .collect();

        tracing::info!("import {} references.", req.len());
        self.sota_repo
            .delete_reference(DeleteRef::DeleteAll)
            .await?;
        self.sota_repo.create_reference(req).await?;
        Ok(())
    }

    async fn update_summit_list(
        &self,
        UploadSOTASummit { data }: UploadSOTASummit,
    ) -> AppResult<()> {
        let partial_equal = |r: &SotaReference, other: &SotaReference| {
            r.summit_code == other.summit_code
                && r.association_name == other.association_name
                && r.region_name == other.region_name
                && r.alt_ft == other.alt_ft
                && r.grid_ref1 == other.grid_ref1
                && r.grid_ref2 == other.grid_ref2
                && r.points == other.points
                && r.bonus_points == other.bonus_points
                && r.valid_from == other.valid_from
                && r.valid_to == other.valid_to
                && r.activation_count == other.activation_count
                && r.activation_date == other.activation_date
                && r.activation_call == other.activation_call
        };

        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;

        let new_hash: HashMap<_, _> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .map(|r| (r.summit_code.clone(), r))
            .collect();

        let query = FindRefBuilder::new().sota().build();
        let result = self.sota_repo.find_reference(&query).await?;
        let old_hash: HashMap<_, _> = result
            .into_iter()
            .map(|r| (r.summit_code.clone(), r))
            .collect();

        let updated: Vec<_> = new_hash
            .keys()
            .cloned()
            .filter_map(|summit_code| {
                let newsummit = new_hash.get(&summit_code).unwrap().clone();
                if old_hash.contains_key(&summit_code) {
                    let oldsummit = old_hash.get(&summit_code).unwrap();
                    if !partial_equal(&newsummit, oldsummit) {
                        Some(newsummit)
                    } else {
                        None
                    }
                } else {
                    Some(newsummit)
                }
            })
            .collect();

        tracing::info!("update summit {} references.", updated.len());

        self.sota_repo.upsert_reference(updated).await?;
        Ok(())
    }

    async fn import_summit_opt_list(
        &self,
        UploadSOTASummitOpt { data }: UploadSOTASummitOpt,
    ) -> AppResult<()> {
        let csv: Vec<SOTASumitOptCSV> = csv_reader(data, false, 1)?;

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
        let requests: Vec<POTACSVFile> = csv_reader(data, false, 1)?;
        let newref = requests.into_iter().map(PotaReference::from).collect();
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
        let requests: Vec<MuniCSVFile> = csv_reader(data, false, 1)?;
        let newtable = requests
            .into_iter()
            .map(MunicipalityCenturyCode::from)
            .collect();
        self.loc_repo.upload_muni_century_list(newtable).await?;
        Ok(())
    }

    async fn show_sota_reference(&self, event: FindRef) -> AppResult<SotaReference> {
        Ok(self.sota_repo.show_reference(&event).await?)
    }

    async fn show_all_sota_references(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>> {
        Ok(self.sota_repo.show_all_references(&event).await?)
    }

    async fn update_sota_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        self.sota_repo.update_reference(references).await?;
        Ok(())
    }

    async fn delete_sota_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        self.sota_repo.delete_reference(event).await?;
        Ok(())
    }

    async fn show_pota_reference(&self, event: FindRef) -> AppResult<PotaReference> {
        Ok(self.pota_repo.show_reference(&event).await?)
    }

    async fn show_all_pota_references(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>> {
        Ok(self.pota_repo.show_all_references(&event).await?)
    }

    async fn update_pota_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
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
