use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/// ログ種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[typeshare]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    #[default]
    Unknown,
    /// アクティベータログ（10カラム）
    Activator,
    /// チェイサーログ（11カラム）
    Chaser,
}

/// 判定モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[typeshare]
#[serde(rename_all = "snake_case")]
pub enum JudgmentMode {
    /// 厳格モード（デフォルト）: アクティベーション日または翌日のいずれかで10局以上
    #[default]
    Strict,
    /// 緩和モード: アクティベーション日 + 翌日の合算で10局以上
    Lenient,
}

/// SOTA日本支部設立10周年記念アワード判定結果
#[derive(Debug, Serialize, Default)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct AwardJudgmentResult {
    pub success: bool,
    /// ログのオペレータコールサイン
    pub callsign: String,
    pub total_qsos: u32,
    pub log_type: LogType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activator: Option<ActivatorAwardResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaser: Option<ChaserAwardResult>,
    pub mode: JudgmentMode,
    /// PDF証明書のダウンロードが可能か
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_available: Option<bool>,
}

/// アクティベータ賞判定結果
/// 条件: 10座の異なる山岳で、それぞれ10局以上の異なる局と交信
#[derive(Debug, Serialize, Default)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct ActivatorAwardResult {
    /// アワード達成かどうか
    pub achieved: bool,
    /// 達成済みの山岳数 (10座以上で達成)
    pub qualified_summits: u32,
    /// 各山岳の詳細
    pub summits: Vec<SummitActivation>,
}

/// 山岳ごとのアクティベーション結果
#[derive(Debug, Serialize, Clone)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SummitActivation {
    /// 山岳コード (例: JA/TK-001)
    pub summit_code: String,
    /// 交信した異なる局の数
    pub unique_stations: u32,
    /// 10局以上で達成
    pub qualified: bool,
}

/// チェイサー賞判定結果
/// 条件: 1つの山岳から10人以上の異なるアクティベータと交信
#[derive(Debug, Serialize, Default)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct ChaserAwardResult {
    /// アワード達成かどうか
    pub achieved: bool,
    /// 達成した山岳のリスト
    pub qualified_summits: Vec<SummitChase>,
}

/// 山岳ごとのチェイス結果
#[derive(Debug, Serialize, Clone)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SummitChase {
    /// 山岳コード (例: JA/TK-001)
    pub summit_code: String,
    /// 交信した異なるアクティベータの数
    pub unique_activators: u32,
    /// アクティベータ一覧
    pub activators: Vec<String>,
}
