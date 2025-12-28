use async_trait::async_trait;
use chrono::Utc;
use shaku::Component;
use std::str::FromStr;
use std::sync::Arc;

use crate::model::pota::{POTAActivatorLogCSV, POTAHunterLogCSV, UploadPOTALog};
use crate::services::PotaLogService;
use common::config::AppConfig;
use common::error::AppResult;
use common::utils::csv_reader;
use domain::model::event::DeleteLog;
use domain::model::id::LogId;
use domain::model::pota::{POTALogKind, PotaLogHist};
use domain::repository::pota::PotaRepository;

#[derive(Component)]
#[shaku(interface = PotaLogService)]
pub struct PotaLogServiceImpl {
    #[shaku(inject)]
    pota_repo: Arc<dyn PotaRepository>,
    config: AppConfig,
}

#[async_trait]
impl PotaLogService for PotaLogServiceImpl {
    async fn upload_pota_log(
        &self,
        UploadPOTALog {
            activator_logid,
            hunter_logid,
            data,
        }: UploadPOTALog,
    ) -> AppResult<PotaLogHist> {
        let logid = if data.contains("Attempts") {
            tracing::info!("Upload activator log");
            activator_logid
        } else {
            tracing::info!("Upload hunter log");
            hunter_logid
        };

        let log_id = LogId::from_str(&logid).unwrap_or(LogId::default());

        let mut update_id = self.pota_repo.find_logid(log_id).await;

        if let Ok(ref mut id) = update_id {
            let expire = Utc::now() - self.config.pota_log_expire;
            if id.update < expire.naive_utc() {
                let query = DeleteLog {
                    log_id: Some(id.log_id),
                    ..Default::default()
                };
                self.pota_repo.delete_log(query).await?;
                *id = PotaLogHist::new(None);
                self.pota_repo.update_logid(id.clone()).await?;
            }
        } else {
            update_id = Ok(PotaLogHist::new(None));
        }

        let mut update_id = update_id?;
        let log_id = update_id.log_id;

        if data.contains("Attempts") {
            let requests: Vec<POTAActivatorLogCSV> = csv_reader(data, false, 1)?;
            let newlog: Vec<_> = requests
                .into_iter()
                .filter_map(|l| POTAActivatorLogCSV::to_log(log_id, l))
                .collect();

            let query = DeleteLog {
                log_id: Some(log_id),
                ..Default::default()
            };
            self.pota_repo.delete_log(query).await?;
            self.pota_repo.upload_activator_log(newlog).await?;

            update_id.log_kind = Some(POTALogKind::ActivatorLog);
        } else {
            let requests: Vec<POTAHunterLogCSV> = csv_reader(data, false, 1)?;
            let newlog: Vec<_> = requests
                .into_iter()
                .filter_map(|l| POTAHunterLogCSV::to_log(log_id, l))
                .collect();

            let query = DeleteLog {
                log_id: Some(log_id),
                ..Default::default()
            };
            self.pota_repo.delete_log(query).await?;
            self.pota_repo.upload_hunter_log(newlog).await?;

            update_id.log_kind = Some(POTALogKind::HunterLog);
        }

        update_id.update = Utc::now().naive_utc();
        self.pota_repo.update_logid(update_id.clone()).await?;

        self.pota_repo
            .delete_log(DeleteLog {
                before: Some(Utc::now() - self.config.pota_log_expire),
                ..Default::default()
            })
            .await?;

        Ok(update_id)
    }

    async fn find_logid(&self, log_id: LogId) -> AppResult<PotaLogHist> {
        self.pota_repo.find_logid(log_id).await
    }

    async fn delete_pota_log(&self, log_id: LogId) -> AppResult<()> {
        self.pota_repo
            .delete_log(DeleteLog {
                log_id: Some(log_id),
                ..Default::default()
            })
            .await?;
        Ok(())
    }
}
