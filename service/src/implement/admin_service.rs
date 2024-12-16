use async_trait::async_trait;
use chrono::{Local, NaiveDate};
use shaku::Component;
use std::sync::Arc;

use common::config::AppConfig;
use common::csv_reader::csv_reader;
use common::error::AppResult;

use domain::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};
use domain::model::pota::{POTAReference, ParkCode};
use domain::model::sota::SOTAReference;
use domain::model::sota::{SOTARefOptInfo, SummitCode};
use domain::repository::{
    healthcheck::HealthCheckRepositry, pota::POTAReferenceRepositry, sota::SOTAReferenceReposity,
};

use crate::model::pota::UploadPOTACSV;
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
    config: AppConfig,
}

#[async_trait]
impl AdminService for AdminServiceImpl {
    async fn import_summit_list(&self, UploadSOTACSV { data }: UploadSOTACSV) -> AppResult<()> {
        let today = Local::now().date_naive();
        let csv: Vec<SOTACSVFile> = csv_reader(data, 2)?;

        let is_valid_summit = |r: &SOTAReference| -> bool {
            let validfrom = NaiveDate::parse_from_str(r.valid_from.as_ref().unwrap(), "%d/%m/%Y")
                .unwrap_or(today);
            let validto = NaiveDate::parse_from_str(r.valid_to.as_ref().unwrap(), "%d/%m/%Y")
                .unwrap_or(today);
            r.summit_code.starts_with("JA") && today <= validto && today >= validfrom
        };
        let req = CreateRef {
            requests: csv
                .into_iter()
                .map(SOTAReference::from)
                .filter(is_valid_summit)
                .collect(),
        };
        eprintln!("import {} references.", req.requests.len());
        self.sota_repo.import_reference(req).await?;
        Ok(())
    }

    async fn import_summit_opt_list(
        &self,
        UploadSOTAOptCSV { data }: UploadSOTAOptCSV,
    ) -> AppResult<()> {
        let csv: Vec<SOTACSVOptFile> = csv_reader(data, 1)?;
        let req = UpdateRef {
            requests: csv.into_iter().map(SOTARefOptInfo::from).collect(),
        };
        self.sota_repo.update_reference_opt(req).await?;
        Ok(())
    }

    async fn import_pota_park_list(&self, UploadPOTACSV { data }: UploadPOTACSV) -> AppResult<()> {
        let requests: Vec<POTAReference> = csv_reader(data, 1)?;
        let req = CreateRef { requests };
        self.pota_repo.import_reference(req).await?;
        Ok(())
    }

    async fn find_sota_reference(&self, event: FindRef) -> AppResult<FindResult<SOTAReference>> {
        Ok(self.sota_repo.find_reference(&event).await?)
    }

    async fn update_sota_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()> {
        self.sota_repo.update_reference_opt(event).await?;
        Ok(())
    }

    async fn delete_sota_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        let req = DeleteRef {
            ref_id: event.ref_id,
        };
        self.sota_repo.delete_reference_opt(req).await?;
        Ok(())
    }

    async fn find_pota_reference(&self, event: FindRef) -> AppResult<FindResult<POTAReference>> {
        Ok(self.pota_repo.find_reference(&event).await?)
    }

    async fn update_pota_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()> {
        self.pota_repo.update_reference(event).await?;
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
