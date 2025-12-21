use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

use common::utils::call_to_operator;

/// 判定モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JudgmentMode {
    /// 厳格モード（デフォルト）: アクティベーション日または翌日のいずれかで10局以上
    #[default]
    Strict,
    /// 緩和モード: アクティベーション日 + 翌日の合算で10局以上
    Lenient,
}

/// ログ種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogType {
    #[default]
    Unknown,
    /// アクティベータログ（10カラム）
    Activator,
    /// チェイサーログ（11カラム）
    Chaser,
}

/// SOTA CSV V2フォーマットログ（in-memory判定用）
#[derive(Debug, Clone, Deserialize)]
pub struct SotaLogEntry {
    pub version: String,
    pub my_callsign: String,
    pub my_summit_code: Option<String>,
    pub date: String,
    pub time: String,
    pub frequency: String,
    pub mode: String,
    pub his_callsign: String,
    pub his_summit_code: Option<String>,
    pub comment: Option<String>,
}

/// アワード判定結果（サービス層）
#[derive(Debug, Default)]
pub struct AwardResult {
    /// ログのオペレータコールサイン
    pub callsign: String,
    pub total_qsos: u32,
    pub log_type: LogType,
    pub activator: Option<ActivatorResult>,
    pub chaser: Option<ChaserResult>,
    pub mode: JudgmentMode,
}

/// アクティベータ賞結果
#[derive(Debug, Default)]
pub struct ActivatorResult {
    pub achieved: bool,
    pub qualified_summits: u32,
    pub summits: Vec<SummitActivationResult>,
}

/// 山岳ごとのアクティベーション結果
#[derive(Debug, Clone)]
pub struct SummitActivationResult {
    pub summit_code: String,
    pub unique_stations: u32,
    pub qualified: bool,
}

/// チェイサー賞結果
#[derive(Debug, Default)]
pub struct ChaserResult {
    pub achieved: bool,
    pub qualified_summits: Vec<SummitChaseResult>,
}

/// 山岳ごとのチェイス結果
#[derive(Debug, Clone)]
pub struct SummitChaseResult {
    pub summit_code: String,
    pub unique_activators: u32,
    pub activators: Vec<String>,
}

/// アワード期間
/// SOTA日本支部設立10周年記念アワード: 2025/6/1 - 2025/12/31 (JST)
pub struct AwardPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl AwardPeriod {
    /// 期間内かどうかを判定
    pub fn contains(&self, dt: DateTime<Utc>) -> bool {
        dt >= self.start && dt < self.end
    }
}

impl Default for AwardPeriod {
    fn default() -> Self {
        // SOTA日本支部設立10周年記念アワード期間: 2025/6/1 - 2025/12/31 (JST)
        // JSTなので UTC+9、UTCでは2025/5/31 15:00 - 2025/12/31 15:00
        Self {
            start: Utc
                .with_ymd_and_hms(2025, 5, 31, 15, 0, 0)
                .single()
                .expect("Invalid award start date"),
            end: Utc
                .with_ymd_and_hms(2025, 12, 31, 15, 0, 0)
                .single()
                .expect("Invalid award end date"),
        }
    }
}

impl SotaLogEntry {
    /// CSVログをパースしてDateTime<Utc>を取得
    pub fn parse_datetime(&self) -> Option<DateTime<Utc>> {
        let date_time = format!("{} {}", self.date, self.time);

        for pat in ["%d/%m/%Y %H:%M", "%d/%m/%Y %H%M"] {
            if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(&date_time, pat) {
                return Some(Utc.from_utc_datetime(&naive));
            }
        }
        None
    }

    /// オペレータ名を取得（/を除いた形式）
    pub fn operator(&self) -> String {
        call_to_operator(&self.my_callsign)
    }

    /// 相手局のオペレータ名を取得
    pub fn his_operator(&self) -> String {
        call_to_operator(&self.his_callsign)
    }

    /// アクティベーションログかどうか
    pub fn is_activation(&self) -> bool {
        self.my_summit_code
            .as_ref()
            .is_some_and(|code| !code.is_empty())
    }

    /// チェイスログかどうか
    pub fn is_chase(&self) -> bool {
        self.his_summit_code
            .as_ref()
            .is_some_and(|code| !code.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_award_period_contains_within() {
        let period = AwardPeriod::default();
        // 2025/7/1 00:00 UTC（期間内）
        let dt = Utc.with_ymd_and_hms(2025, 7, 1, 0, 0, 0).unwrap();
        assert!(period.contains(dt));
    }

    #[test]
    fn test_award_period_contains_start_boundary() {
        let period = AwardPeriod::default();
        // 開始日時ちょうど（含まれる）
        assert!(period.contains(period.start));
    }

    #[test]
    fn test_award_period_contains_end_boundary() {
        let period = AwardPeriod::default();
        // 終了日時ちょうど（含まれない）
        assert!(!period.contains(period.end));
    }

    #[test]
    fn test_award_period_contains_before() {
        let period = AwardPeriod::default();
        // 2025/1/1（期間前）
        let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert!(!period.contains(dt));
    }

    #[test]
    fn test_award_period_contains_after() {
        let period = AwardPeriod::default();
        // 2026/1/1（期間後）
        let dt = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert!(!period.contains(dt));
    }

    #[test]
    fn test_sota_log_entry_parse_datetime() {
        let entry = SotaLogEntry {
            version: "V2".to_string(),
            my_callsign: "JA1ABC".to_string(),
            my_summit_code: Some("JA/TK-001".to_string()),
            date: "01/07/2025".to_string(),
            time: "12:34".to_string(),
            frequency: "7.032".to_string(),
            mode: "CW".to_string(),
            his_callsign: "JA2XYZ".to_string(),
            his_summit_code: None,
            comment: None,
        };
        let dt = entry.parse_datetime().unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 7);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 34);
    }

    #[test]
    fn test_sota_log_entry_is_activation() {
        let entry = SotaLogEntry {
            version: "V2".to_string(),
            my_callsign: "JA1ABC".to_string(),
            my_summit_code: Some("JA/TK-001".to_string()),
            date: "01/07/2025".to_string(),
            time: "12:34".to_string(),
            frequency: "7.032".to_string(),
            mode: "CW".to_string(),
            his_callsign: "JA2XYZ".to_string(),
            his_summit_code: None,
            comment: None,
        };
        assert!(entry.is_activation());
    }

    #[test]
    fn test_sota_log_entry_is_chase() {
        let entry = SotaLogEntry {
            version: "V2".to_string(),
            my_callsign: "JA1ABC".to_string(),
            my_summit_code: None,
            date: "01/07/2025".to_string(),
            time: "12:34".to_string(),
            frequency: "7.032".to_string(),
            mode: "CW".to_string(),
            his_callsign: "JA2XYZ".to_string(),
            his_summit_code: Some("JA/TK-002".to_string()),
            comment: None,
        };
        assert!(entry.is_chase());
        assert!(!entry.is_activation());
    }

    #[test]
    fn test_sota_log_entry_empty_summit_code() {
        let entry = SotaLogEntry {
            version: "V2".to_string(),
            my_callsign: "JA1ABC".to_string(),
            my_summit_code: Some("".to_string()), // 空文字
            date: "01/07/2025".to_string(),
            time: "12:34".to_string(),
            frequency: "7.032".to_string(),
            mode: "CW".to_string(),
            his_callsign: "JA2XYZ".to_string(),
            his_summit_code: Some("".to_string()), // 空文字
            comment: None,
        };
        assert!(!entry.is_activation());
        assert!(!entry.is_chase());
    }
}
