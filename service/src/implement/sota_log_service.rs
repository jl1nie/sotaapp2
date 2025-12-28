use async_trait::async_trait;
use chrono::Utc;
use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::award_calculator::{detect_log_type, judge_award_with_mode};
use crate::model::award::{AwardPeriod, AwardResult, JudgmentMode, SotaLogEntry};
use crate::model::sota::{SOTALogCSV, UploadSOTALog};
use crate::services::SotaLogService;
use common::error::AppResult;
use common::utils::csv_reader;
use domain::model::event::{DeleteLog, FindLog};
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

    async fn award_progress(&self, _user_id: UserId, mut query: FindLog) -> AppResult<String> {
        let mut response = String::new();

        let after = query.after.unwrap_or_default();
        let before = query.before.unwrap_or_default();

        query.activation = true;
        let act_log = self.sota_repo.find_log(&query).await?;
        query.activation = false;
        let chase_log = self.sota_repo.find_log(&query).await?;

        let mut act_hash = HashMap::new();

        for a in act_log {
            let Some(summit_code) = a.my_summit_code else {
                continue;
            };
            let date = a.time.date_naive();
            let newent = act_hash
                .entry((a.operator, summit_code))
                .or_insert(HashMap::new());
            let act_cnt = newent.entry(date).or_insert(0);
            *act_cnt += 1;
        }

        let mut chase_summit_hash = HashMap::new();

        for c in chase_log {
            let Some(his_summit_code) = c.his_summit_code else {
                continue;
            };
            let his_callsign = common::utils::call_to_operator(&c.his_callsign);

            let newent = chase_summit_hash
                .entry(c.operator)
                .or_insert(HashMap::new());

            let act_call = newent.entry(his_summit_code).or_insert(HashSet::new());
            act_call.insert(his_callsign);
        }

        // 各(operator, summit_code)ペアについて、アクティベーション達成(10QSO以上)した最初の日を見つける
        let act_result: Vec<_> = act_hash
            .into_iter()
            .filter_map(|((call, summit_code), date_counts)| {
                let mut dates: Vec<_> = date_counts.keys().collect();
                if dates.is_empty() {
                    return None;
                }
                dates.sort();

                // 4QSO以上達成した最初の日を探す
                let mut first_activation_date = None;
                let mut first_activation_qso_count = 0;

                for date in dates {
                    if let Some(&count) = date_counts.get(date) {
                        if count >= 10 {
                            first_activation_date = Some(date);
                            first_activation_qso_count = count;
                            break; // 最初の成功したアクティベーションを見つけたらループを抜ける
                        }
                    }
                }

                // 4QSO以上のアクティベーションが見つからなければフィルタリング
                first_activation_date?;

                // 最初の成功したアクティベーションのQSOだけを返す
                Some((call, (first_activation_qso_count, summit_code)))
            })
            .collect();

        let mut act_hash = HashMap::new();
        for (call, result) in act_result {
            act_hash.entry(call).or_insert(Vec::new()).push(result);
        }

        // 10局以上のQSOがあるアクティベーションのみフィルタリング
        let act_result: Vec<_> = act_hash
            .into_iter()
            .filter(|(_, summits)| summits.iter().any(|(qso_count, _)| *qso_count >= 10))
            .collect();

        response.push_str(&format!("集計期間 {} - {}\n", after, before));
        for mut a in act_result {
            if a.1.len() >= 10 {
                response.push_str(&format!(
                    "アクティベータ：{} activate {} summits ",
                    a.0,
                    a.1.len()
                ));
                a.1.sort_by(|a, b| b.0.cmp(&a.0));
                for s in a.1 {
                    response.push_str(&format!("{} {}qsos, ", s.1, s.0));
                }
                response.push('\n');
            }
        }

        let chase_summit10: Vec<_> = chase_summit_hash
            .clone()
            .into_iter()
            .map(|(call, mut summit_hash)| {
                summit_hash.retain(|_, acts| acts.len() >= 5);
                (call, summit_hash)
            })
            .filter(|(_, summit_hash)| !summit_hash.len() >= 10)
            .collect();

        for (call, summit_hash) in chase_summit10 {
            response.push_str(&format!(
                "チェイサー10x10 {} {} summits {:?}\n",
                call,
                summit_hash.len(),
                summit_hash
            ));
        }

        tracing::info!("Result = {}", response);

        Ok(response)
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
