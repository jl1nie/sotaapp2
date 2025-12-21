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
use crate::model::pota::{POTAAllCSVFile, POTACSVFile, UploadPOTAReference};
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
    ) -> AppResult<usize> {
        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;
        let req: Vec<_> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .collect();

        let count = req.len();
        tracing::info!("import {} references.", count);
        self.sota_repo
            .delete_reference(DeleteRef::DeleteAll)
            .await?;

        self.sota_repo.create_reference(req).await?;

        Ok(count)
    }

    async fn update_summit_list(
        &self,
        UploadSOTASummit { data }: UploadSOTASummit,
    ) -> AppResult<usize> {
        let partial_equal = |r: &SotaReference, other: &SotaReference| {
            r.activation_count == other.activation_count
                && r.activation_date == other.activation_date
                && r.activation_call == other.activation_call
                && r.summit_code == other.summit_code
                && r.association_name == other.association_name
                && r.region_name == other.region_name
                && r.alt_ft == other.alt_ft
                && r.grid_ref1 == other.grid_ref1
                && r.grid_ref2 == other.grid_ref2
                && r.points == other.points
                && r.bonus_points == other.bonus_points
                && r.valid_from == other.valid_from
                && r.valid_to == other.valid_to
        };

        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;

        tracing::info!("Latest summit list length = {}", csv.len());

        let mut new_hash: HashMap<_, _> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .map(|r| (r.summit_code.clone(), r))
            .collect();

        let limit = 5000;
        let mut offset = 0;

        loop {
            let query = FindRefBuilder::new()
                .sota()
                .limit(limit)
                .offset(offset)
                .build();
            let result = self.sota_repo.find_reference(&query).await?;

            if result.is_empty() {
                break;
            }

            for r in result {
                let n = new_hash.get(&r.summit_code);
                if let Some(summit) = n {
                    if partial_equal(&r, summit) {
                        new_hash.remove(&r.summit_code);
                    }
                }
            }
            offset += limit;
        }

        let updated: Vec<_> = new_hash.into_values().collect();
        let count = updated.len();

        tracing::info!("update {} summits.", count);
        self.sota_repo.upsert_reference(updated).await?;

        Ok(count)
    }

    async fn import_summit_opt_list(
        &self,
        UploadSOTASummitOpt { data }: UploadSOTASummitOpt,
    ) -> AppResult<usize> {
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

        let mut total_count = 0;
        for assoc in associations {
            let query = FindRefBuilder::new().sota().name(assoc).build();
            let result = self.sota_repo.find_reference(&query).await?;
            let newref: Vec<_> = result
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
            total_count += newref.len();
            self.sota_repo.update_reference(newref).await?;
        }

        Ok(total_count)
    }

    async fn import_pota_park_list(
        &self,
        UploadPOTAReference { data }: UploadPOTAReference,
    ) -> AppResult<usize> {
        let requests: Vec<POTAAllCSVFile> = csv_reader(data, false, 1)?;
        let newref: Vec<_> = requests
            .into_iter()
            .filter_map(|r| PotaReference::try_from(r).ok())
            .filter(|r| !r.pota_code.starts_with("JP-"))
            .collect();

        let count = newref.len();
        tracing::info!("update {} parks.", count);
        self.pota_repo.create_reference(newref).await?;

        Ok(count)
    }

    async fn import_pota_park_list_ja(
        &self,
        UploadPOTAReference { data }: UploadPOTAReference,
    ) -> AppResult<usize> {
        let requests: Vec<POTACSVFile> = csv_reader(data, false, 1)?;
        let newref: Vec<_> = requests.into_iter().map(PotaReference::from).collect();

        let count = newref.len();
        tracing::info!("update {} JA parks.", count);
        self.pota_repo.create_reference(newref).await?;

        Ok(count)
    }

    async fn import_muni_century_list(
        &self,
        UploadMuniCSV { data }: UploadMuniCSV,
    ) -> AppResult<usize> {
        let requests: Vec<MuniCSVFile> = csv_reader(data, false, 1)?;
        let newtable: Vec<_> = requests
            .into_iter()
            .map(MunicipalityCenturyCode::from)
            .collect();
        let count = newtable.len();
        self.loc_repo.upload_muni_century_list(newtable).await?;

        Ok(count)
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
