use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typeshare::typeshare;
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

use common::utils::maidenhead;

/// 既定のズームレベル
pub const DEFAULT_ZOOM: u8 = 15;

/// 地図ページのパス
pub const MAP_PATH: &str = "/api/v2/map";

/// 地図タイルの種別
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "lowercase")]
pub enum TileLayer {
    /// OpenStreetMap（全世界）
    #[default]
    Osm,
    /// 地理院タイル（日本国内）
    Gsi,
}

impl TileLayer {
    /// タイルのURLテンプレート
    pub fn url_template(&self) -> &'static str {
        match self {
            TileLayer::Osm => "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
            TileLayer::Gsi => "https://cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png",
        }
    }

    /// タイル提供元の表示（著作権表示）
    pub fn attribution(&self) -> &'static str {
        match self {
            TileLayer::Osm => {
                r#"&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors"#
            }
            TileLayer::Gsi => {
                r#"<a href="https://maps.gsi.go.jp/development/ichiran.html">地理院タイル</a>"#
            }
        }
    }

    /// タイルが提供される最大ズームレベル
    pub fn max_zoom(&self) -> u8 {
        match self {
            TileLayer::Osm => 19,
            TileLayer::Gsi => 18,
        }
    }

    /// クエリパラメータでの表記
    pub fn as_query_value(&self) -> &'static str {
        match self {
            TileLayer::Osm => "osm",
            TileLayer::Gsi => "gsi",
        }
    }
}

/// クエリ文字列に埋め込む値をパーセントエンコードする
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

/// 有限の数値であることを検証する
///
/// `range`バリデーションはNaNや無限大を素通ししてしまうため、別途チェックする。
fn validate_finite(value: f64) -> Result<(), ValidationError> {
    if value.is_finite() {
        Ok(())
    } else {
        let mut error = ValidationError::new("not_finite");
        error.message = Some(Cow::from("座標には有限の数値を指定してください"));
        Err(error)
    }
}

/// 地図表示APIのクエリパラメータ
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema, IntoParams)]
pub struct MapParam {
    /// 緯度
    #[validate(
        range(min = -90.0, max = 90.0, message = "緯度は-90〜90の範囲で指定してください"),
        custom(function = "validate_finite")
    )]
    pub lat: f64,
    /// 経度
    #[validate(
        range(min = -180.0, max = 180.0, message = "経度は-180〜180の範囲で指定してください"),
        custom(function = "validate_finite")
    )]
    pub lon: f64,
    /// ズームレベル（省略時は15）
    #[validate(range(
        min = 1,
        max = 19,
        message = "ズームレベルは1〜19の範囲で指定してください"
    ))]
    pub zoom: Option<u8>,
    /// マーカーに表示する名称
    #[validate(length(max = 100, message = "labelは100文字以内で指定してください"))]
    pub label: Option<String>,
    /// 地図タイルの種別（省略時はosm）
    pub tile: Option<TileLayer>,
    /// 埋め込み表示（ヘッダを省略。iframeでの利用を想定）
    pub embed: Option<bool>,
}

impl MapParam {
    /// 使用するタイル種別
    pub fn tile_layer(&self) -> TileLayer {
        self.tile.unwrap_or_default()
    }

    /// 埋め込み表示かどうか
    pub fn is_embed(&self) -> bool {
        self.embed.unwrap_or(false)
    }

    /// 地図ページのURLを組み立てる
    ///
    /// `embed`が真の場合はヘッダ非表示のURL（iframe埋め込み用）を返す。
    pub fn to_url(&self, embed: bool) -> String {
        let mut url = format!(
            "{}?lat={}&lon={}&zoom={}&tile={}",
            MAP_PATH,
            self.lat,
            self.lon,
            self.effective_zoom(),
            self.tile_layer().as_query_value(),
        );
        if let Some(label) = self.label.as_deref().filter(|l| !l.is_empty()) {
            url.push_str(&format!("&label={}", percent_encode(label)));
        }
        if embed {
            url.push_str("&embed=true");
        }
        url
    }

    /// タイルの最大ズームに丸めた実効ズームレベル
    pub fn effective_zoom(&self) -> u8 {
        let zoom = self.zoom.unwrap_or(DEFAULT_ZOOM);
        zoom.min(self.tile_layer().max_zoom()).max(1)
    }
}

/// 地図タイルの情報（フロントエンドが自前で地図を描画する場合に使用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct TileLayerView {
    /// タイル種別（osm / gsi）
    pub kind: TileLayer,
    /// Leaflet等にそのまま渡せるURLテンプレート
    pub url_template: String,
    /// 著作権表示（HTML）
    pub attribution: String,
    /// タイルが提供される最大ズームレベル
    pub max_zoom: u8,
}

/// 外部地図サービスへのリンク
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct ExternalMapLinks {
    pub google_maps: String,
    pub open_street_map: String,
    pub gsi_maps: String,
}

/// 地図表示に必要な情報一式
///
/// フロントエンドが自前の地図コンポーネントで描画する場合や、
/// `embedUrl`をiframeに埋め込む場合に使用する。
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct MapView {
    /// 中心の緯度
    pub latitude: f64,
    /// 中心の経度
    pub longitude: f64,
    /// 実効ズームレベル
    pub zoom: u8,
    /// グリッドロケータ（メイデンヘッド）
    pub maidenhead: String,
    /// マーカーに表示する名称
    pub label: Option<String>,
    /// タイル情報
    pub tile: TileLayerView,
    /// 地図ページのURL（別タブで開く用）
    pub map_url: String,
    /// 埋め込み用URL（iframe用。ヘッダ非表示）
    pub embed_url: String,
    /// 外部地図サービスへのリンク
    pub external_links: ExternalMapLinks,
}

impl From<&MapParam> for MapView {
    fn from(param: &MapParam) -> Self {
        let (lat, lon) = (param.lat, param.lon);
        let zoom = param.effective_zoom();
        let tile = param.tile_layer();

        Self {
            latitude: lat,
            longitude: lon,
            zoom,
            maidenhead: maidenhead(lon, lat),
            label: param.label.clone(),
            tile: TileLayerView {
                kind: tile,
                url_template: tile.url_template().to_string(),
                attribution: tile.attribution().to_string(),
                max_zoom: tile.max_zoom(),
            },
            map_url: param.to_url(false),
            embed_url: param.to_url(true),
            external_links: ExternalMapLinks {
                google_maps: format!("https://www.google.com/maps?q={lat},{lon}"),
                open_street_map: format!(
                    "https://www.openstreetmap.org/?mlat={lat}&mlon={lon}#map={zoom}/{lat}/{lon}"
                ),
                gsi_maps: format!("https://maps.gsi.go.jp/#{zoom}/{lat}/{lon}/"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(zoom: Option<u8>, tile: Option<TileLayer>) -> MapParam {
        MapParam {
            lat: 35.360_556,
            lon: 138.727_778,
            zoom,
            label: None,
            tile,
            embed: None,
        }
    }

    #[test]
    fn test_default_tile_is_osm() {
        assert_eq!(param(None, None).tile_layer(), TileLayer::Osm);
    }

    #[test]
    fn test_default_zoom() {
        assert_eq!(param(None, None).effective_zoom(), DEFAULT_ZOOM);
    }

    #[test]
    fn test_zoom_clamped_to_tile_max_zoom() {
        // 地理院タイルは18まで
        assert_eq!(param(Some(19), Some(TileLayer::Gsi)).effective_zoom(), 18);
        assert_eq!(param(Some(19), Some(TileLayer::Osm)).effective_zoom(), 19);
    }

    #[test]
    fn test_validate_accepts_valid_coordinates() {
        assert!(param(Some(15), None).validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_out_of_range_latitude() {
        let mut p = param(None, None);
        p.lat = 91.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_out_of_range_longitude() {
        let mut p = param(None, None);
        p.lon = -181.0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_nan_coordinates() {
        let mut p = param(None, None);
        p.lat = f64::NAN;
        assert!(p.validate().is_err());

        let mut p = param(None, None);
        p.lon = f64::NAN;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_infinite_coordinates() {
        let mut p = param(None, None);
        p.lat = f64::INFINITY;
        assert!(p.validate().is_err());

        let mut p = param(None, None);
        p.lon = f64::NEG_INFINITY;
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_out_of_range_zoom() {
        assert!(param(Some(20), None).validate().is_err());
        assert!(param(Some(0), None).validate().is_err());
    }

    #[test]
    fn test_validate_rejects_too_long_label() {
        let mut p = param(None, None);
        p.label = Some("あ".repeat(101));
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("富士山"), "%E5%AF%8C%E5%A3%AB%E5%B1%B1");
        assert_eq!(percent_encode("JA/TK-001"), "JA%2FTK-001");
        assert_eq!(percent_encode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(percent_encode("Aa0-_.~"), "Aa0-_.~");
    }

    #[test]
    fn test_to_url_includes_encoded_label() {
        let mut p = param(Some(12), Some(TileLayer::Gsi));
        p.label = Some("富士山 JA/SO-001".to_string());

        assert_eq!(
            p.to_url(false),
            "/api/v2/map?lat=35.360556&lon=138.727778&zoom=12&tile=gsi&label=%E5%AF%8C%E5%A3%AB%E5%B1%B1%20JA%2FSO-001"
        );
        assert!(p.to_url(true).ends_with("&embed=true"));
    }

    #[test]
    fn test_to_url_omits_empty_label() {
        let mut p = param(None, None);
        p.label = Some(String::new());
        assert!(!p.to_url(false).contains("label"));
    }

    #[test]
    fn test_map_view_uses_effective_zoom() {
        // 地理院タイルの最大ズーム(18)に丸められた値がviewにも反映される
        let view = MapView::from(&param(Some(19), Some(TileLayer::Gsi)));
        assert_eq!(view.zoom, 18);
        assert_eq!(view.tile.max_zoom, 18);
        assert!(view.map_url.contains("zoom=18"));
    }

    #[test]
    fn test_tile_deserialized_from_value() {
        let p: MapParam = serde_json::from_str(r#"{"lat":35.0,"lon":139.0,"tile":"gsi"}"#)
            .expect("MapParamをデシリアライズできること");
        assert_eq!(p.tile_layer(), TileLayer::Gsi);
    }
}
