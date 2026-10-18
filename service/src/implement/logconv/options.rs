//! 変換のオプションと、変換の結果

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 自局の参照を読む場所
///
/// 画面の「運用先」の指定(`myQTH`)。API では `rmks1` / `rmks2` / `form`。未知の値は `form` として扱う(旧版と同じ)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MyQth {
    /// 備考1の列
    Rmks1,
    /// 備考2の列
    Rmks2,
    /// 画面で入力した参照(`Summit` / `Park`)
    #[default]
    #[serde(other)]
    Form,
}

/// 相手局の参照を読む場所
///
/// 画面の「相手局」の指定(`QTH`)。API では `none` / `rmks1` / `rmks2` / `qth`。未知の値は `none` として扱う(旧版と同じ)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HisQth {
    /// 備考1の列
    Rmks1,
    /// 備考2の列
    Rmks2,
    /// QTH の列
    Qth,
    /// 指定しない
    #[default]
    #[serde(other)]
    None,
}

/// Conversion options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionOptions {
    /// 自局の参照を読む場所
    pub my_qth: MyQth,
    /// 相手局の参照を読む場所
    pub his_qth: HisQth,
    /// 自局の山の参照(`my_qth` が `Form` のとき。SOTA アクティベーター)
    pub summit: String,
    /// 自局の公園の参照(`my_qth` が `Form` のとき。ADIF)。`JAFF-0005 JP-0014` のように複数を書ける
    pub park: Vec<String>,
    /// SOTA activator callsign
    pub sota_activator: String,
    /// POTA activator callsign
    pub pota_activator: String,
    /// POTA operator callsign
    pub pota_operator: String,
    /// 公園(POTA)の所在地。ADIF の MY_STATE になる(例: `JP-0117` → `TK`)。引けなかった公園は入っていない
    #[serde(default)]
    pub park_states: std::collections::HashMap<String, String>,
}

/// Conversion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub status: String,
    pub error_log: Vec<String>,
    pub log_text: Vec<Vec<String>>,
    pub file_list: Vec<String>,
    pub files: HashMap<String, String>,
}

impl Default for ConversionResult {
    fn default() -> Self {
        Self {
            status: "OK".to_string(),
            error_log: Vec::new(),
            log_text: Vec::new(),
            file_list: Vec::new(),
            files: HashMap::new(),
        }
    }
}
