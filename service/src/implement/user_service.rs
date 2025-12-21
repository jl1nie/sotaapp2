use async_trait::async_trait;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use domain::model::AwardProgram;
use regex::Regex;
use shaku::Component;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use crate::model::award::{
    ActivatorResult, AwardPeriod, AwardResult, ChaserResult, JudgmentMode, LogType, SotaLogEntry,
    SummitActivationResult, SummitChaseResult,
};
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
                                    if !p.is_empty() {
                                        let pota = p.first().unwrap();
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
                .map(|l| POTAActivatorLogCSV::to_log(log_id, l))
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
                .map(|l| POTAHunterLogCSV::to_log(log_id, l))
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

        let from = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| SOTALogCSV::to_log(user_id.clone(), l))
            .filter(|l| l.time >= from && l.time < to)
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
            let summit_code = a.my_summit_code.unwrap();
            let date = a.time.date_naive();
            let newent = act_hash
                .entry((a.operator, summit_code))
                .or_insert(HashMap::new());
            let act_cnt = newent.entry(date).or_insert(0);
            *act_cnt += 1;
        }

        let mut chase_summit_hash = HashMap::new();

        for c in chase_log {
            //let my_operator = c.operator.clone();
            let his_summit_code = c.his_summit_code.unwrap();
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
                if first_activation_date.is_none() {
                    return None;
                }

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

    fn judge_10th_anniversary_award(&self, csv_data: &str, mode: JudgmentMode) -> AppResult<AwardResult> {
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

/// ログ種別を自動判定（カラム数で判断）
/// - 10カラム: アクティベータログ
/// - 11カラム: チェイサーログ
fn detect_log_type(csv_data: &str) -> LogType {
    if let Some(first_line) = csv_data.lines().next() {
        let column_count = first_line.split(',').count();
        match column_count {
            10 => LogType::Activator,
            11 => LogType::Chaser,
            _ => LogType::Unknown,
        }
    } else {
        LogType::Unknown
    }
}

/// In-memoryでアワード判定を行う（ログ種別とモード指定）
fn judge_award_with_mode(
    logs: Vec<SotaLogEntry>,
    period: &AwardPeriod,
    mode: JudgmentMode,
    log_type: LogType,
) -> AwardResult {
    // 最初のログエントリからコールサインを取得
    let callsign = logs.first().map(|l| l.operator()).unwrap_or_default();

    let mut total_qsos = 0u32;

    // アクティベータ: 山岳コード -> UTC日付 -> 交信した局のセット
    let mut activator_map: HashMap<String, BTreeMap<NaiveDate, HashSet<String>>> = HashMap::new();

    // チェイサー: 山岳コード -> アクティベータのセット
    let mut chaser_map: HashMap<String, HashSet<String>> = HashMap::new();

    for log in logs {
        // 日時をパース
        let datetime = match log.parse_datetime() {
            Some(dt) => dt,
            None => continue,
        };

        // 期間外のログはスキップ
        if datetime < period.start || datetime >= period.end {
            continue;
        }

        total_qsos += 1;

        // アクティベーションログの処理（アクティベータログの場合のみ）
        if log_type == LogType::Activator && log.is_activation() {
            let summit_code = log.my_summit_code.as_ref().unwrap().to_uppercase();
            let his_operator = log.his_operator().to_uppercase();
            let utc_date = datetime.date_naive();

            activator_map
                .entry(summit_code)
                .or_default()
                .entry(utc_date)
                .or_default()
                .insert(his_operator);
        }

        // チェイスログの処理（チェイサーログの場合のみ）
        if log_type == LogType::Chaser && log.is_chase() {
            let his_summit_code = log.his_summit_code.as_ref().unwrap().to_uppercase();
            let his_operator = log.his_operator().to_uppercase();

            chaser_map
                .entry(his_summit_code)
                .or_default()
                .insert(his_operator);
        }
    }

    // アクティベータ賞の判定（アクティベータログの場合のみ）
    let activator = if log_type == LogType::Activator {
        let mut summits: Vec<SummitActivationResult> = activator_map
            .into_iter()
            .map(|(summit_code, date_map)| evaluate_summit_activation(&summit_code, date_map, mode))
            .collect();

        // ユニーク局数で降順ソート
        summits.sort_by(|a, b| b.unique_stations.cmp(&a.unique_stations));

        let qualified_summits = summits.iter().filter(|s| s.qualified).count() as u32;
        Some(ActivatorResult {
            achieved: qualified_summits >= 10,
            qualified_summits,
            summits,
        })
    } else {
        None
    };

    // チェイサー賞の判定（チェイサーログの場合のみ）
    let chaser = if log_type == LogType::Chaser {
        let mut qualified_chase_summits: Vec<SummitChaseResult> = chaser_map
            .into_iter()
            .filter_map(|(summit_code, activators)| {
                let unique_activators = activators.len() as u32;
                if unique_activators >= 10 {
                    let mut activator_list: Vec<String> = activators.into_iter().collect();
                    activator_list.sort();
                    Some(SummitChaseResult {
                        summit_code,
                        unique_activators,
                        activators: activator_list,
                    })
                } else {
                    None
                }
            })
            .collect();

        // ユニークアクティベータ数で降順ソート
        qualified_chase_summits.sort_by(|a, b| b.unique_activators.cmp(&a.unique_activators));

        Some(ChaserResult {
            // チェイサー賞: 1つの山から10人以上のアクティベータと交信で達成
            achieved: !qualified_chase_summits.is_empty(),
            qualified_summits: qualified_chase_summits,
        })
    } else {
        None
    };

    AwardResult {
        callsign,
        total_qsos,
        log_type,
        activator,
        chaser,
        mode,
    }
}

/// 山岳ごとのアクティベーション評価
/// - 最初に4局以上達成した日をアクティベーション日とする
/// - アクティベーション日とその翌日のみを評価対象とする
/// - 厳格モード: いずれかの日で10局以上
/// - 緩和モード: 2日間の合算で10局以上
fn evaluate_summit_activation(
    summit_code: &str,
    date_map: BTreeMap<NaiveDate, HashSet<String>>,
    mode: JudgmentMode,
) -> SummitActivationResult {
    let dates: Vec<_> = date_map.keys().cloned().collect();

    // アクティベーション日を探す（最初に4局以上達成した日）
    let mut activation_date: Option<NaiveDate> = None;
    for date in &dates {
        if let Some(stations) = date_map.get(date) {
            if stations.len() >= 4 {
                activation_date = Some(*date);
                break;
            }
        }
    }

    // アクティベーション日が見つからない場合（4局未満）
    let Some(act_date) = activation_date else {
        // 全日の合計を返す（未達成）
        let all_stations: HashSet<_> = date_map.values().flatten().cloned().collect();
        return SummitActivationResult {
            summit_code: summit_code.to_string(),
            unique_stations: all_stations.len() as u32,
            qualified: false,
        };
    };

    // アクティベーション日の局
    let day1_stations = date_map.get(&act_date).cloned().unwrap_or_default();

    // 翌日の局（連続している場合のみ）
    let next_date = act_date + Duration::days(1);
    let day2_stations = date_map.get(&next_date).cloned().unwrap_or_default();

    // モードに応じて判定
    let (unique_stations, qualified) = match mode {
        JudgmentMode::Strict => {
            // 厳格モード: いずれかの日で10局以上
            let day1_count = day1_stations.len();
            let day2_count = day2_stations.len();

            if day1_count >= 10 || day2_count >= 10 {
                // どちらかで達成
                let max_count = day1_count.max(day2_count);
                (max_count as u32, true)
            } else {
                // 2日間のユニーク局数を返す（参考情報として）
                let combined: HashSet<_> = day1_stations
                    .iter()
                    .chain(day2_stations.iter())
                    .cloned()
                    .collect();
                (combined.len() as u32, false)
            }
        }
        JudgmentMode::Lenient => {
            // 緩和モード: 2日間の合算で10局以上
            let combined: HashSet<_> = day1_stations
                .iter()
                .chain(day2_stations.iter())
                .cloned()
                .collect();
            let count = combined.len();
            (count as u32, count >= 10)
        }
    };

    SummitActivationResult {
        summit_code: summit_code.to_string(),
        unique_stations,
        qualified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::award::AwardPeriod;

    fn make_log(
        my_summit: Option<&str>,
        his_callsign: &str,
        his_summit: Option<&str>,
        date: &str,
    ) -> SotaLogEntry {
        SotaLogEntry {
            version: "V2".to_string(),
            my_callsign: "JH1ABC".to_string(),
            my_summit_code: my_summit.map(|s| s.to_string()),
            date: date.to_string(),
            time: "1000".to_string(),
            frequency: "14.280".to_string(),
            mode: "SSB".to_string(),
            his_callsign: his_callsign.to_string(),
            his_summit_code: his_summit.map(|s| s.to_string()),
            comment: None,
        }
    }

    fn test_period() -> AwardPeriod {
        AwardPeriod {
            start: Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_activator_not_qualified() {
        // 1山岳で5局のみ（10局未満）
        let logs = vec![
            make_log(Some("JA/TK-001"), "JH2XYZ", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH3ABC", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH4DEF", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH5GHI", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH6JKL", None, "01/07/2025"),
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert_eq!(result.total_qsos, 5);
        assert!(!activator.achieved);
        assert_eq!(activator.qualified_summits, 0);
        assert_eq!(activator.summits.len(), 1);
        assert_eq!(activator.summits[0].unique_stations, 5);
        assert!(!activator.summits[0].qualified);
    }

    #[test]
    fn test_activator_one_summit_qualified() {
        // 1山岳で10局（達成だが、10座必要なのでアワード未達成）
        let logs: Vec<SotaLogEntry> = (0..10)
            .map(|i| make_log(Some("JA/TK-001"), &format!("JH{}XYZ", i), None, "01/07/2025"))
            .collect();

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert_eq!(result.total_qsos, 10);
        assert!(!activator.achieved); // 10座必要
        assert_eq!(activator.qualified_summits, 1);
        assert!(activator.summits[0].qualified);
    }

    #[test]
    fn test_activator_full_achievement() {
        // 10山岳でそれぞれ10局ずつ
        let mut logs = Vec::new();
        for summit_idx in 0..10 {
            for station_idx in 0..10 {
                logs.push(make_log(
                    Some(&format!("JA/TK-{:03}", summit_idx + 1)),
                    &format!("JH{}S{}", summit_idx, station_idx),
                    None,
                    "01/07/2025",
                ));
            }
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert_eq!(result.total_qsos, 100);
        assert!(activator.achieved);
        assert_eq!(activator.qualified_summits, 10);
    }

    #[test]
    fn test_chaser_not_qualified() {
        // 1山岳から3人のアクティベータ（10人未満）
        let logs = vec![
            make_log(None, "JH2XYZ/P", Some("JA/NN-001"), "01/07/2025"),
            make_log(None, "JH3ABC/P", Some("JA/NN-001"), "01/07/2025"),
            make_log(None, "JH4DEF/P", Some("JA/NN-001"), "01/07/2025"),
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

        let chaser = result.chaser.unwrap();
        assert_eq!(result.total_qsos, 3);
        assert!(!chaser.achieved);
        assert!(chaser.qualified_summits.is_empty());
    }

    #[test]
    fn test_chaser_qualified() {
        // 1山岳から10人のアクティベータ
        let logs: Vec<SotaLogEntry> = (0..10)
            .map(|i| {
                make_log(
                    None,
                    &format!("JH{}/P", i),
                    Some("JA/NN-001"),
                    "01/07/2025",
                )
            })
            .collect();

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

        let chaser = result.chaser.unwrap();
        assert_eq!(result.total_qsos, 10);
        assert!(chaser.achieved);
        assert_eq!(chaser.qualified_summits.len(), 1);
        assert_eq!(chaser.qualified_summits[0].unique_activators, 10);
    }

    #[test]
    fn test_duplicate_callsigns_counted_once() {
        // 同じ局と複数回交信しても1局としてカウント
        let logs = vec![
            make_log(Some("JA/TK-001"), "JH2XYZ", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH2XYZ", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH2XYZ", None, "01/07/2025"),
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert_eq!(result.total_qsos, 3);
        assert_eq!(activator.summits[0].unique_stations, 1);
    }

    #[test]
    fn test_chaser_same_activator_different_days_counted_once() {
        // 同じサミットで同じアクティベータと異なる日に交信しても1回としてカウント
        let logs = vec![
            make_log(None, "JH2XYZ/P", Some("JA/NN-001"), "01/07/2025"),
            make_log(None, "JH2XYZ/P", Some("JA/NN-001"), "03/07/2025"), // 同じアクティベータ、別の日
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

        // chaser_mapはサミットごとにアクティベータをHashSetで管理するため、
        // 同じアクティベータは1回としてカウントされる
        // ただし10人未満なのでqualified_summitsには含まれない
        let chaser = result.chaser.unwrap();
        assert_eq!(result.total_qsos, 2);
        assert!(!chaser.achieved);
        // qualified_summitsは10人以上のみなので空
        assert!(chaser.qualified_summits.is_empty());
    }

    #[test]
    fn test_chaser_same_activator_different_summits_counted_separately() {
        // 異なるサミットで同じアクティベータと交信した場合は、各サミットで別々にカウント
        // サミットAで10人、サミットBでも同じ10人と交信 → 両方で達成
        let mut logs = Vec::new();
        // サミットAで10人のアクティベータ
        for i in 0..10 {
            logs.push(make_log(
                None,
                &format!("JH{}/P", i),
                Some("JA/NN-001"),
                "01/07/2025",
            ));
        }
        // サミットBで同じ10人のアクティベータ
        for i in 0..10 {
            logs.push(make_log(
                None,
                &format!("JH{}/P", i),
                Some("JA/NN-002"),
                "02/07/2025",
            ));
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

        let chaser = result.chaser.unwrap();
        assert_eq!(result.total_qsos, 20);
        assert!(chaser.achieved);
        // 両方のサミットで達成
        assert_eq!(chaser.qualified_summits.len(), 2);
    }

    #[test]
    fn test_out_of_period_excluded() {
        // 期間外のログは除外
        let logs = vec![
            make_log(Some("JA/TK-001"), "JH2XYZ", None, "01/05/2025"), // 期間前
            make_log(Some("JA/TK-001"), "JH3ABC", None, "01/07/2025"), // 期間内
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert_eq!(result.total_qsos, 1);
        assert_eq!(activator.summits[0].unique_stations, 1);
    }

    // ====== 日付境界テスト ======

    #[test]
    fn test_strict_mode_day1_4qso_day2_10qso_qualified() {
        // 厳格モード: Day1で4局、Day2で10局 → Day2で達成
        let mut logs = Vec::new();
        // Day1: 4局（アクティベーション成立）
        for i in 0..4 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}AAA", i),
                None,
                "01/07/2025",
            ));
        }
        // Day2: 10局
        for i in 0..10 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}BBB", i),
                None,
                "02/07/2025",
            ));
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert!(activator.summits[0].qualified);
        assert_eq!(activator.summits[0].unique_stations, 10);
    }

    #[test]
    fn test_strict_mode_day1_4qso_day2_6qso_not_qualified() {
        // 厳格モード: Day1で4局、Day2で6局 → どちらも10局未満で不達成
        let mut logs = Vec::new();
        // Day1: 4局
        for i in 0..4 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}AAA", i),
                None,
                "01/07/2025",
            ));
        }
        // Day2: 6局
        for i in 0..6 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}BBB", i),
                None,
                "02/07/2025",
            ));
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert!(!activator.summits[0].qualified);
        // 参考情報として2日間のユニーク局数を返す
        assert_eq!(activator.summits[0].unique_stations, 10);
    }

    #[test]
    fn test_lenient_mode_day1_4qso_day2_6qso_qualified() {
        // 緩和モード: Day1で4局、Day2で6局 → 合算10局で達成
        let mut logs = Vec::new();
        // Day1: 4局
        for i in 0..4 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}AAA", i),
                None,
                "01/07/2025",
            ));
        }
        // Day2: 6局
        for i in 0..6 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}BBB", i),
                None,
                "02/07/2025",
            ));
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Lenient, LogType::Activator);

        let activator = result.activator.unwrap();
        assert!(activator.summits[0].qualified);
        assert_eq!(activator.summits[0].unique_stations, 10);
    }

    #[test]
    fn test_non_consecutive_days_not_merged() {
        // 連続しない日は合算されない
        let mut logs = Vec::new();
        // Day1: 4局
        for i in 0..4 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}AAA", i),
                None,
                "01/07/2025",
            ));
        }
        // Day3（1日空き）: 10局
        for i in 0..10 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}BBB", i),
                None,
                "03/07/2025", // 2日後
            ));
        }

        // 厳格モードでも緩和モードでも、Day3は評価対象外
        let result_strict =
            judge_award_with_mode(logs.clone(), &test_period(), JudgmentMode::Strict, LogType::Activator);
        let result_lenient = judge_award_with_mode(logs, &test_period(), JudgmentMode::Lenient, LogType::Activator);

        let activator_strict = result_strict.activator.unwrap();
        let activator_lenient = result_lenient.activator.unwrap();
        // Day1のみ評価され、10局未満なので不達成
        assert!(!activator_strict.summits[0].qualified);
        assert!(!activator_lenient.summits[0].qualified);
        assert_eq!(activator_strict.summits[0].unique_stations, 4);
        assert_eq!(activator_lenient.summits[0].unique_stations, 4);
    }

    #[test]
    fn test_one_activation_per_summit_rule() {
        // 同一山岳は最初のアクティベーションのみ評価
        let mut logs = Vec::new();
        // Day1: 4局（アクティベーション成立、10局未満）
        for i in 0..4 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}AAA", i),
                None,
                "01/07/2025",
            ));
        }
        // 1ヶ月後: 10局（2回目のアクティベーション、年1回ルールで無視）
        for i in 0..10 {
            logs.push(make_log(
                Some("JA/TK-001"),
                &format!("JH{}BBB", i),
                None,
                "01/08/2025",
            ));
        }

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        // Day1のアクティベーションのみ評価、翌日がないので4局のみ
        assert!(!activator.summits[0].qualified);
        assert_eq!(activator.summits[0].unique_stations, 4);
    }

    #[test]
    fn test_no_activation_if_less_than_4qso() {
        // 4局未満はアクティベーション不成立
        let logs = vec![
            make_log(Some("JA/TK-001"), "JH1AAA", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH2BBB", None, "01/07/2025"),
            make_log(Some("JA/TK-001"), "JH3CCC", None, "01/07/2025"),
        ];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Activator);

        let activator = result.activator.unwrap();
        assert!(!activator.summits[0].qualified);
        assert_eq!(activator.summits[0].unique_stations, 3);
    }

    #[test]
    fn test_default_mode_is_strict() {
        // デフォルトは厳格モード
        let logs = vec![make_log(Some("JA/TK-001"), "JH1AAA", None, "01/07/2025")];

        let result = judge_award_with_mode(logs, &test_period(), JudgmentMode::default(), LogType::Activator);

        assert_eq!(result.mode, JudgmentMode::Strict);
    }
}
