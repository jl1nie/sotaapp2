//! SOTA日本支部設立10周年記念アワード判定ロジック
//!
//! このモジュールは、アワード達成判定のための純粋関数を提供します。
//! データベースアクセスは不要で、CSVログデータのin-memory判定を行います。

use chrono::{Duration, NaiveDate};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::award::{
    ActivatorResult, AwardPeriod, AwardResult, ChaserResult, JudgmentMode, LogType, SotaLogEntry,
    SummitActivationResult, SummitChaseResult,
};

/// ログ種別を自動判定（カラム数で判断）
/// - 10カラム: アクティベータログ
/// - 11カラム: チェイサーログ
pub fn detect_log_type(csv_data: &str) -> LogType {
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
pub fn judge_award_with_mode(
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
            if let Some(summit_code) = log.my_summit_code.as_ref() {
                let summit_code = summit_code.to_uppercase();
                let his_operator = log.his_operator().to_uppercase();
                let utc_date = datetime.date_naive();

                activator_map
                    .entry(summit_code)
                    .or_default()
                    .entry(utc_date)
                    .or_default()
                    .insert(his_operator);
            }
        }

        // チェイスログの処理（チェイサーログの場合のみ）
        if log_type == LogType::Chaser && log.is_chase() {
            let Some(his_summit_code) = log.his_summit_code.as_ref() else {
                continue;
            };
            let his_summit_code = his_summit_code.to_uppercase();
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
    use chrono::{TimeZone, Utc};

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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
            .map(|i| {
                make_log(
                    Some("JA/TK-001"),
                    &format!("JH{}XYZ", i),
                    None,
                    "01/07/2025",
                )
            })
            .collect();

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result =
            judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

        let chaser = result.chaser.unwrap();
        assert_eq!(result.total_qsos, 3);
        assert!(!chaser.achieved);
        assert!(chaser.qualified_summits.is_empty());
    }

    #[test]
    fn test_chaser_qualified() {
        // 1山岳から10人のアクティベータ
        let logs: Vec<SotaLogEntry> = (0..10)
            .map(|i| make_log(None, &format!("JH{}/P", i), Some("JA/NN-001"), "01/07/2025"))
            .collect();

        let result =
            judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result =
            judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

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

        let result =
            judge_award_with_mode(logs, &test_period(), JudgmentMode::Strict, LogType::Chaser);

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Lenient,
            LogType::Activator,
        );

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
        let result_strict = judge_award_with_mode(
            logs.clone(),
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );
        let result_lenient = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Lenient,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

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

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::Strict,
            LogType::Activator,
        );

        let activator = result.activator.unwrap();
        assert!(!activator.summits[0].qualified);
        assert_eq!(activator.summits[0].unique_stations, 3);
    }

    #[test]
    fn test_default_mode_is_strict() {
        // デフォルトは厳格モード
        let logs = vec![make_log(Some("JA/TK-001"), "JH1AAA", None, "01/07/2025")];

        let result = judge_award_with_mode(
            logs,
            &test_period(),
            JudgmentMode::default(),
            LogType::Activator,
        );

        assert_eq!(result.mode, JudgmentMode::Strict);
    }
}
