use async_trait::async_trait;
use chrono::Utc;
use shaku::Component;
use std::sync::Arc;

use super::award_calculator::{detect_log_type, judge_award_with_mode};
use crate::model::award::{AwardPeriod, AwardResult, JudgmentMode, SotaLogEntry};
use crate::model::sota::{SOTALogCSV, UploadSOTALog};
use crate::services::SotaLogService;
use common::error::AppResult;
use common::utils::csv_reader;
use domain::model::event::DeleteLog;
use domain::model::id::UserId;
use domain::repository::sota::SotaRepository;

#[derive(Component)]
#[shaku(interface = SotaLogService)]
pub struct SotaLogServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
}

#[async_trait]
impl SotaLogService for SotaLogServiceImpl {
    async fn upload_sota_log(
        &self,
        user_id: UserId,
        UploadSOTALog { data }: UploadSOTALog,
    ) -> AppResult<()> {
        let requests: Vec<SOTALogCSV> = csv_reader(data, false, 0)?;

        let period = AwardPeriod::default();

        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| SOTALogCSV::to_log(user_id.clone(), l))
            .filter(|l| period.contains(l.time))
            .collect();
        self.sota_repo.upload_log(newlog).await?;
        Ok(())
    }

    async fn delete_sota_log(&self, _user_id: UserId) -> AppResult<()> {
        self.sota_repo
            .delete_log(DeleteLog {
                before: Some(Utc::now()),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    fn judge_10th_anniversary_award(
        &self,
        csv_data: &str,
        mode: JudgmentMode,
    ) -> AppResult<AwardResult> {
        // ログ種別を自動判定（最初の行のカラム数で判断）
        let log_type = detect_log_type(csv_data);
        tracing::info!("Detected log type: {:?}", log_type);

        // CSVをパース（ヘッダーなし、スキップ0行）
        let logs: Vec<SotaLogEntry> = csv_reader(csv_data.to_string(), false, 0)?;

        tracing::info!(
            "Judging 10th anniversary award: {} log entries parsed, mode={:?}, log_type={:?}",
            logs.len(),
            mode,
            log_type
        );

        // アワード期間を設定
        let period = AwardPeriod::default();

        // in-memoryで判定（ログ種別とモード指定）
        let result = judge_award_with_mode(logs, &period, mode, log_type);

        tracing::info!(
            "Award judgment complete: {} QSOs in period, log_type={:?}",
            result.total_qsos,
            result.log_type
        );

        Ok(result)
    }
}
