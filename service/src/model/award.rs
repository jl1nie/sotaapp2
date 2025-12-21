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
pub struct AwardPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Default for AwardPeriod {
    fn default() -> Self {
        // SOTA日本支部設立10周年記念アワード期間: 2025/6/1 - 2025/12/31 (JST)
        // JSTなので UTC+9、UTCでは2025/5/31 15:00 - 2025/12/31 14:59:59
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
