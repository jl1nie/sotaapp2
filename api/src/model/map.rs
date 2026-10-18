use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typeshare::typeshare;
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

use common::utils::{calculate_distance, maidenhead};
use domain::model::event::FindResult;

/// 既定のズームレベル
pub const DEFAULT_ZOOM: u8 = 15;

/// 近傍検索の既定の半径(m)
pub const DEFAULT_NEARBY_RADIUS_M: u32 = 1000;

/// 近傍として返す件数の上限
pub const MAX_NEARBY: usize = 20;

/// 地図ページのパス
pub const MAP_PATH: &str = "/api/v2/map";

/// 地図タイルの種別
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "lowercase")]
pub enum TileLayer {
    /// OpenStreetMap（全世界）
    Osm,
    /// 地理院タイル（日本国内。既定）
    #[default]
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
    /// 地図タイルの種別（省略時はgsi。日本国外はosmを指定）
    pub tile: Option<TileLayer>,
    /// 埋め込み表示（ヘッダを省略。iframeでの利用を想定）
    pub embed: Option<bool>,
    /// 近傍のSOTA山岳・POTA公園を表示する（省略時はtrue）
    pub refs: Option<bool>,
    /// 近傍検索の半径。単位はメートル（1〜10000、省略時は1000）
    #[validate(range(
        min = 1,
        max = 10000,
        message = "radiusは1〜10000m(メートル)の範囲で指定してください"
    ))]
    pub radius: Option<u32>,
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

    /// 近傍のリファレンスを検索するか
    pub fn want_refs(&self) -> bool {
        self.refs.unwrap_or(true)
    }

    /// 近傍検索の半径(m)
    pub fn radius_m(&self) -> u32 {
        self.radius.unwrap_or(DEFAULT_NEARBY_RADIUS_M)
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
        if !self.want_refs() {
            url.push_str("&refs=false");
        }
        if let Some(radius) = self.radius {
            url.push_str(&format!("&radius={radius}"));
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

/// リファレンスの種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    Sota,
    Pota,
}

/// 地図の中心の近くにあるSOTA山岳・POTA公園
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct NearbyRef {
    pub kind: RefKind,
    /// リファレンスコード（例: JA/TK-001, JP-0001/JA-0001）
    pub code: String,
    pub name: String,
    pub name_j: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    /// 中心からの距離(m)
    pub distance_m: u32,
    /// 標高(m)。SOTAのみ
    pub alt_m: Option<i32>,
    /// ポイント。SOTAのみ
    pub points: Option<i32>,
    /// 所在地。POTAのみ
    pub location: Option<String>,
}

impl NearbyRef {
    /// 検索結果から、中心から`radius_m`以内のものを近い順に最大`MAX_NEARBY`件返す
    ///
    /// DBの検索は正方形の範囲で絞るため、ここで距離により円に絞り直す。
    pub fn collect(result: FindResult, lat: f64, lon: f64, radius_m: u32) -> Vec<NearbyRef> {
        let mut refs = Vec::new();

        for r in result.sota.unwrap_or_default() {
            refs.push(NearbyRef {
                kind: RefKind::Sota,
                code: r.summit_code,
                name: r.summit_name,
                name_j: r.summit_name_j,
                latitude: r.latitude,
                longitude: r.longitude,
                distance_m: 0,
                alt_m: Some(r.alt_m),
                points: Some(r.points),
                location: None,
            });
        }

        for r in result.pota.unwrap_or_default() {
            let code = match (r.pota_code.as_str(), r.wwff_code.as_str()) {
                ("", wwff) => wwff.to_string(),
                (pota, "") => pota.to_string(),
                (pota, wwff) => format!("{}/{}", pota, wwff),
            };
            refs.push(NearbyRef {
                kind: RefKind::Pota,
                code,
                name: r.park_name,
                name_j: Some(r.park_name_j).filter(|n| !n.is_empty()),
                latitude: r.latitude,
                longitude: r.longitude,
                distance_m: 0,
                alt_m: None,
                points: None,
                location: Some(r.park_location).filter(|l| !l.is_empty()),
            });
        }

        for r in refs.iter_mut() {
            r.distance_m = calculate_distance(lat, lon, r.latitude, r.longitude).round() as u32;
        }
        refs.retain(|r| r.distance_m <= radius_m);
        refs.sort_by_key(|r| r.distance_m);
        refs.truncate(MAX_NEARBY);
        refs
    }
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
    /// 近傍のSOTA山岳・POTA公園（近い順）
    pub nearby: Vec<NearbyRef>,
}

impl MapView {
    /// 近傍のリファレンスを設定する
    pub fn with_nearby(mut self, nearby: Vec<NearbyRef>) -> Self {
        self.nearby = nearby;
        self
    }
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
            nearby: Vec::new(),
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
            refs: None,
            radius: None,
        }
    }

    #[test]
    fn test_default_tile_is_gsi() {
        assert_eq!(param(None, None).tile_layer(), TileLayer::Gsi);
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

    fn sota(code: &str, lat: f64, lon: f64) -> domain::model::sota::SotaReference {
        domain::model::sota::SotaReference {
            summit_code: code.to_string(),
            association_name: String::new(),
            region_name: String::new(),
            summit_name: code.to_string(),
            summit_name_j: None,
            city: None,
            city_j: None,
            alt_m: 1000,
            alt_ft: 3281,
            grid_ref1: String::new(),
            grid_ref2: String::new(),
            longitude: lon,
            latitude: lat,
            maidenhead: String::new(),
            points: 6,
            bonus_points: 0,
            valid_from: chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            valid_to: chrono::NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            activation_count: 0,
            activation_date: None,
            activation_call: None,
        }
    }

    fn pota(code: &str, lat: f64, lon: f64) -> domain::model::pota::PotaRefLog {
        domain::model::pota::PotaRefLog {
            pota_code: code.to_string(),
            wwff_code: String::new(),
            park_name: code.to_string(),
            park_name_j: String::new(),
            park_location: String::new(),
            park_locid: String::new(),
            park_type: String::new(),
            park_inactive: false,
            park_area: 0,
            longitude: lon,
            latitude: lat,
            maidenhead: String::new(),
            attempts: None,
            activations: None,
            first_qso_date: None,
            qsos: None,
        }
    }

    #[test]
    fn test_collect_filters_by_circle_and_sorts_by_distance() {
        let (lat, lon) = (35.0, 139.0);
        // 北に 500m / 900m / 1500m、北東の箱の角(約 1200m)にある点
        // (緯度1度 ≒ 111.2km として近似。許容誤差は距離の判定に十分)
        let m = 1.0 / 111_195.0;
        let north = |d: f64| (lat + d * m, lon);
        let (a, b, c) = (north(500.0), north(900.0), north(1500.0));
        let off = 1200.0 * std::f64::consts::FRAC_1_SQRT_2;
        let corner = (lat + off * m, lon + off * m / lat.to_radians().cos());
        let result = FindResult {
            sota: Some(vec![sota("FAR", c.0, c.1), sota("NEAR", a.0, a.1)]),
            pota: Some(vec![
                pota("PARK", b.0, b.1),
                pota("CORNER", corner.0, corner.1),
            ]),
        };
        let refs = NearbyRef::collect(result, lat, lon, 1000);
        let codes: Vec<_> = refs.iter().map(|r| r.code.as_str()).collect();
        // 半径1000m以内のみ、近い順
        assert_eq!(codes, ["NEAR", "PARK"]);
        assert!((refs[0].distance_m as i64 - 500).abs() <= 1);
        assert_eq!(refs[0].kind, RefKind::Sota);
        assert_eq!(refs[0].alt_m, Some(1000));
        assert_eq!(refs[1].kind, RefKind::Pota);
        assert_eq!(refs[1].alt_m, None);
    }

    #[test]
    fn test_collect_limits_count() {
        let sotas = (0..MAX_NEARBY + 5)
            .map(|i| sota(&format!("S{i}"), 35.0 + i as f64 * 1e-5, 139.0))
            .collect();
        let result = FindResult {
            sota: Some(sotas),
            pota: None,
        };
        assert_eq!(
            NearbyRef::collect(result, 35.0, 139.0, 1000).len(),
            MAX_NEARBY
        );
    }

    #[test]
    fn test_pota_code_combines_wwff() {
        let mut p = pota("JP-0001", 35.0, 139.0);
        p.wwff_code = "JAFF-0001".to_string();
        let result = FindResult {
            sota: None,
            pota: Some(vec![p]),
        };
        let refs = NearbyRef::collect(result, 35.0, 139.0, 1000);
        assert_eq!(refs[0].code, "JP-0001/JAFF-0001");
    }

    #[test]
    fn test_nearby_defaults_and_url() {
        let p = param(None, None);
        assert!(p.want_refs());
        assert_eq!(p.radius_m(), DEFAULT_NEARBY_RADIUS_M);
        let mut p = param(None, None);
        p.refs = Some(false);
        p.radius = Some(500);
        let url = p.to_url(false);
        assert!(url.contains("&refs=false") && url.contains("&radius=500"));
    }
}
