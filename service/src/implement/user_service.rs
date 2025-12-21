use async_trait::async_trait;
use chrono::Utc;
use domain::model::AwardProgram;
use regex::Regex;
use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use super::award_calculator::{detect_log_type, judge_award_with_mode};
use crate::model::award::{AwardPeriod, AwardResult, JudgmentMode, SotaLogEntry};
use crate::model::pota::{POTAActivatorLogCSV, POTAHunterLogCSV, UploadPOTALog};
use crate::model::sota::{SOTALogCSV, UploadSOTALog};
use crate::services::UserService;
use common::config::AppConfig;
use common::error::AppResult;
use common::utils::csv_reader;
use domain::model::activation::{Alert, Spot, SpotLog};
use domain::model::aprslog::{AprsLog, AprsTrack};
use domain::model::event::{
    DeleteLog, FindAct, FindAprs, FindLog, FindRef, FindRefBuilder, FindResult, GroupBy,
};
use domain::model::geomag::GeomagIndex;
use domain::model::id::{LogId, UserId};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{POTALogKind, PotaLogHist};
use domain::repository::{
    activation::ActivationRepositry, aprs::AprsLogRepository, geomag::GeoMagRepositry,
    locator::LocatorRepositry, pota::PotaRepository, sota::SotaRepository,
};

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
    #[shaku(inject)]
    pota_repo: Arc<dyn PotaRepository>,
    #[shaku(inject)]
    pub act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    locator_repo: Arc<dyn LocatorRepositry>,
    #[shaku(inject)]
    pub aprs_log_repo: Arc<dyn AprsLogRepository>,
    #[shaku(inject)]
    geomag_repo: Arc<dyn GeoMagRepositry>,
    config: AppConfig,
}

fn get_alert_group(event: &FindAct, r: &Alert) -> GroupBy {
    if let Some(g) = &event.group_by {
        match g {
            GroupBy::Callsign(_) => GroupBy::Callsign(Some(r.activator.clone())),
            GroupBy::Reference(_) => GroupBy::Reference(Some(r.reference.clone())),
        }
    } else {
        GroupBy::Callsign(None)
    }
}

fn get_spot_group(event: &FindAct, r: &Spot) -> GroupBy {
    if let Some(g) = &event.group_by {
        match g {
            GroupBy::Callsign(_) => GroupBy::Callsign(Some(r.activator.clone())),
            GroupBy::Reference(_) => GroupBy::Reference(Some(r.reference.clone())),
        }
    } else {
        GroupBy::Callsign(None)
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn count_references(&self, event: &FindRef) -> AppResult<i64> {
        let mut result = 0i64;

        if event.is_sota() {
            result += self.sota_repo.count_reference(event).await?;
        }

        if event.is_pota() {
            result += self.pota_repo.count_reference(event).await?;
        }

        Ok(result)
    }

    async fn find_references(&self, event: FindRef) -> AppResult<FindResult> {
        let mut result = FindResult::default();

        if event.is_sota() {
            let sota_ref = self.sota_repo.find_reference(&event).await?;
            result.sota = Some(sota_ref)
        }

        if event.is_pota() {
            let active_ref: Vec<_> = self
                .pota_repo
                .find_reference(&event)
                .await?
                .into_iter()
                .filter(|r| !r.park_inactive)
                .collect();
            result.pota = Some(active_ref)
        }

        Ok(result)
    }

    async fn find_alerts(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Alert>>> {
        let mut result = HashMap::new();
        if event.group_by.is_some() {
            let mut alerts = self.act_repo.find_alerts(&event).await?;
            if let Some(loc_regex) = &event.pattern {
                let pat = Regex::new(loc_regex);
                if let Ok(pat) = pat {
                    alerts.retain(|r| pat.is_match(&r.location));
                }
            }
            for alert in alerts {
                result
                    .entry(get_alert_group(&event, &alert))
                    .or_insert(Vec::new())
                    .push(alert);
            }
        }
        Ok(result)
    }

    async fn find_spots(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<SpotLog>>> {
        let mut result = HashMap::new();
        if event.group_by.is_some() {
            let mut spots = self.act_repo.find_spots(&event).await?;

            if let Some(loc_regex) = &event.pattern {
                let pat = Regex::new(loc_regex);
                if let Ok(pat) = pat {
                    spots.retain(|r| pat.is_match(&r.reference));
                }
            }
            let mut pota_hash: HashMap<String, SpotLog> = HashMap::new();

            for spot in spots {
                let mut spotlog = SpotLog::new(spot.clone(), None);

                match get_spot_group(&event, &spot) {
                    GroupBy::Callsign(_) => {
                        result
                            .entry(get_spot_group(&event, &spot))
                            .or_insert(Vec::new())
                            .push(spotlog);
                    }
                    GroupBy::Reference(code) => {
                        if spot.program == AwardProgram::POTA && event.log_id.is_some() {
                            let code = code.unwrap_or_default();
                            if let Some(v) = pota_hash.get(&code) {
                                spotlog.qsos = v.qsos;
                            } else {
                                let builder = FindRefBuilder::default();
                                let query = builder
                                    .pota()
                                    .pota_code(code.clone())
                                    .log_id(event.log_id.unwrap())
                                    .build();

                                let parks = self.find_references(query).await?;
                                if let FindResult { pota: Some(p), .. } = parks {
                                    if let Some(pota) = p.first() {
                                        spotlog.qsos = pota.qsos;
                                        pota_hash.insert(code, spotlog.clone());
                                    }
                                }
                            }
                            result
                                .entry(get_spot_group(&event, &spot))
                                .or_insert(Vec::new())
                                .push(spotlog);
                        } else {
                            result
                                .entry(get_spot_group(&event, &spot))
                                .or_insert(Vec::new())
                                .push(spotlog);
                        }
                    }
                }
            }
        }
        Ok(result)
    }

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

    async fn get_geomagnetic(&self) -> AppResult<Option<GeomagIndex>> {
        Ok(self.geomag_repo.get_geomag().await?)
    }

    async fn find_aprs_log(&self, event: FindAprs) -> AppResult<Vec<AprsLog>> {
        Ok(self.aprs_log_repo.find_aprs_log(&event).await?)
    }

    async fn get_aprs_track(&self, event: FindAprs) -> AppResult<Vec<AprsTrack>> {
        let aprslog = self.find_aprs_log(event).await?;
        self.generate_track(aprslog).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::award::LogType;
    use domain::model::activation::Spot;
    use domain::model::event::{FindActBuilder, GroupBy};
    use domain::model::AwardProgram;

    /// テスト用Alertを生成するヘルパー
    fn make_test_alert(activator: &str, reference: &str) -> Alert {
        Alert {
            program: AwardProgram::SOTA,
            alert_id: 1,
            user_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            location: "Tokyo".to_string(),
            start_time: Utc::now(),
            end_time: None,
            frequencies: "14.280".to_string(),
            comment: Some("Test".to_string()),
            poster: Some(activator.to_string()),
        }
    }

    /// テスト用Spotを生成するヘルパー
    fn make_test_spot(activator: &str, reference: &str) -> Spot {
        Spot {
            program: AwardProgram::SOTA,
            spot_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            spot_time: Utc::now(),
            frequency: "14.280".to_string(),
            mode: "SSB".to_string(),
            spotter: "JA2XYZ".to_string(),
            comment: Some("Test".to_string()),
        }
    }

    // ==================== ヘルパー関数テスト ====================

    #[test]
    fn test_get_alert_group_by_callsign() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_callsign(Some("JA1ABC".to_string()))
            .build();

        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Callsign(Some(call)) => assert_eq!(call, "JA1ABC"),
            _ => panic!("Expected GroupBy::Callsign"),
        }
    }

    #[test]
    fn test_get_alert_group_by_reference() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_reference(Some("JA/TK-001".to_string()))
            .build();

        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Reference(Some(ref_code)) => assert_eq!(ref_code, "JA/TK-001"),
            _ => panic!("Expected GroupBy::Reference"),
        }
    }

    #[test]
    fn test_get_alert_group_no_group_defaults_to_callsign() {
        let event = FindActBuilder::default().sota().build();
        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Callsign(None) => {} // デフォルトはCallsign(None)
            _ => panic!("Expected GroupBy::Callsign(None)"),
        }
    }

    #[test]
    fn test_get_spot_group_by_callsign() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_callsign(Some("JA1ABC".to_string()))
            .build();

        let spot = make_test_spot("JA1ABC", "JA/TK-001");
        let group = get_spot_group(&event, &spot);

        match group {
            GroupBy::Callsign(Some(call)) => assert_eq!(call, "JA1ABC"),
            _ => panic!("Expected GroupBy::Callsign"),
        }
    }

    #[test]
    fn test_get_spot_group_by_reference() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_reference(Some("JA/TK-001".to_string()))
            .build();

        let spot = make_test_spot("JA1ABC", "JA/TK-001");
        let group = get_spot_group(&event, &spot);

        match group {
            GroupBy::Reference(Some(ref_code)) => assert_eq!(ref_code, "JA/TK-001"),
            _ => panic!("Expected GroupBy::Reference"),
        }
    }

    // ==================== 10周年アワード判定テスト ====================

    #[test]
    fn test_judge_10th_anniversary_award_activator_csv() {
        // 10カラムのアクティベータログ
        let csv = r#"V2,JA1ABC/P,JA/TK-001,01/07/2025,1000,14.280,SSB,JA2XYZ,,
V2,JA1ABC/P,JA/TK-001,01/07/2025,1001,14.280,SSB,JA3DEF,,
V2,JA1ABC/P,JA/TK-001,01/07/2025,1002,14.280,SSB,JA4GHI,,
V2,JA1ABC/P,JA/TK-001,01/07/2025,1003,14.280,SSB,JA5JKL,,
"#;

        let log_type = detect_log_type(csv);
        assert_eq!(log_type, LogType::Activator);

        let logs: Vec<SotaLogEntry> = csv_reader(csv.to_string(), false, 0).unwrap();
        let period = AwardPeriod::default();
        let result = judge_award_with_mode(logs, &period, JudgmentMode::Strict, log_type);

        assert_eq!(result.total_qsos, 4);
        assert!(result.activator.is_some());
        assert!(result.chaser.is_none());

        let activator = result.activator.unwrap();
        assert!(!activator.achieved); // 10座必要だが1座のみ
        assert_eq!(activator.summits.len(), 1);
        assert_eq!(activator.summits[0].unique_stations, 4);
    }

    #[test]
    fn test_judge_10th_anniversary_award_chaser_csv() {
        // 11カラムのチェイサーログ（末尾にポイント列）
        // 実際のフォーマット: V2,コール,空欄,日付,時刻,周波数,モード,相手コール,リファレンス,コメント,ポイント
        let csv = r#"V2,JA1ABC,,01/07/2025,1000,14.280,SSB,JA2XYZ/P,JA/TK-001,FB 59!,10
V2,JA1ABC,,01/07/2025,1001,14.280,SSB,JA3DEF/P,JA/TK-001,,4
"#;

        let log_type = detect_log_type(csv);
        assert_eq!(log_type, LogType::Chaser);

        let logs: Vec<SotaLogEntry> = csv_reader(csv.to_string(), false, 0).unwrap();
        let period = AwardPeriod::default();
        let result = judge_award_with_mode(logs, &period, JudgmentMode::Strict, log_type);

        assert_eq!(result.total_qsos, 2);
        assert!(result.activator.is_none());
        assert!(result.chaser.is_some());

        let chaser = result.chaser.unwrap();
        assert!(!chaser.achieved); // 10人のアクティベータが必要だが2人のみ
    }
}
