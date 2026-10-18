//! リクエストのオプション(`options` の JSON)

use serde::Deserialize;
use service::implement::logconv::ConversionOptions;

/// 出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// SOTA アクティベーター(CSV と ADIF)
    SotaA,
    /// SOTA チェイサー(CSV)
    SotaC,
    /// POTA・WWFF・SOTA の ADIF(自局の参照ごとのファイル)
    Pota,
}

/// 変換のオプション。画面が送る名前(`myQTH` など)のまま受け取る
#[derive(Debug, Deserialize, Default)]
pub struct ConversionOptionsRequest {
    /// 自局の参照を読む場所 (rmks1, rmks2, form)
    #[serde(rename = "myQTH")]
    pub my_qth: Option<String>,
    /// 相手局の参照を読む場所 (rmks1, rmks2, qth, none)
    #[serde(rename = "QTH")]
    pub his_qth: Option<String>,
    /// ユーザー定義ロケーション
    #[serde(rename = "Location")]
    pub location: Option<String>,
    /// SOTAサミットリファレンス(自局の参照を `form` で入力したとき)
    #[serde(rename = "Summit")]
    pub summit: Option<String>,
    /// POTAパークリファレンス(同上。`JAFF-0005 JP-0014` のように複数を書ける)
    #[serde(rename = "Park")]
    pub park: Option<String>,
    /// SOTAアクティベーターコールサイン
    #[serde(rename = "SOTAActivator")]
    pub sota_activator: Option<String>,
    /// POTAアクティベーターコールサイン
    #[serde(rename = "POTAActivator")]
    pub pota_activator: Option<String>,
    /// POTAオペレーターコールサイン
    #[serde(rename = "POTAOperator")]
    pub pota_operator: Option<String>,
}

impl From<ConversionOptionsRequest> for ConversionOptions {
    fn from(req: ConversionOptionsRequest) -> Self {
        ConversionOptions {
            my_qth: req.my_qth.unwrap_or_else(|| "rmks1".to_string()),
            his_qth: req.his_qth.unwrap_or_else(|| "qth".to_string()),
            location: req.location.unwrap_or_default(),
            summit: req.summit.unwrap_or_default(),
            park: req.park.map(|p| vec![p]).unwrap_or_default(),
            sota_activator: req.sota_activator.unwrap_or_default(),
            pota_activator: req.pota_activator.unwrap_or_default(),
            pota_operator: req.pota_operator.unwrap_or_default(),
            ..Default::default()
        }
    }
}
