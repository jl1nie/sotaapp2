use async_trait::async_trait;
use chrono::{Local, NaiveDate};
use common::config::AppConfig;
use common::error::AppResult;
use domain::model::common::event::CreateRef;
use shaku::Component;
use std::sync::Arc;

use domain::model::pota::{event::UploadPOTACSV, POTAReference, ParkCode};
use domain::model::sota::{
    event::{UploadSOTACSV, UploadSOTAOptCSV},
    SOTARefOptInfo, SummitCode,
};
use domain::model::{
    common::{
        csv_reader::csv_reader,
        event::{DeleteRef, FindRef, FindResult, UpdateRef},
    },
    sota::SOTAReference,
};

use crate::model::sota::{SOTACSVFile, SOTACSVOptFile};

use data_access::interface::{HealthCheck, POTADatabase, SOTADatabase};

use crate::interface::AdminService;

#[derive(Component)]
#[shaku(interface = AdminService)]
pub struct AdminServiceImpl {
    #[shaku(inject)]
    sota_db: Arc<dyn SOTADatabase>,
    #[shaku(inject)]
    pota_db: Arc<dyn POTADatabase>,
    #[shaku(inject)]
    check_db: Arc<dyn HealthCheck>,
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
        self.sota_db.import_reference(req).await?;
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
        self.sota_db.update_reference_opt(req).await?;
        Ok(())
    }

    async fn import_pota_park_list(&self, UploadPOTACSV { data }: UploadPOTACSV) -> AppResult<()> {
        let requests: Vec<POTAReference> = csv_reader(data, 1)?;
        let req = CreateRef { requests };
        self.pota_db.import_reference(req).await?;
        Ok(())
    }

    async fn find_sota_reference(&self, event: FindRef) -> AppResult<FindResult<SOTAReference>> {
        Ok(self.sota_db.find_reference(&event).await?)
    }

    async fn update_sota_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()> {
        self.sota_db.update_reference_opt(event).await?;
        Ok(())
    }

    async fn delete_sota_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        let req = DeleteRef {
            ref_id: event.ref_id,
        };
        self.sota_db.delete_reference_opt(req).await?;
        Ok(())
    }

    async fn find_pota_reference(&self, event: FindRef) -> AppResult<FindResult<POTAReference>> {
        Ok(self.pota_db.find_reference(&event).await?)
    }

    async fn update_pota_reference(&self, event: UpdateRef<POTAReference>) -> AppResult<()> {
        self.pota_db.update_reference(event).await?;
        Ok(())
    }

    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        self.pota_db.delete_reference(event).await?;
        Ok(())
    }

    async fn health_check(&self) -> AppResult<bool> {
        Ok(self.check_db.check_database().await?)
    }
}
