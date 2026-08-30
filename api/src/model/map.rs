use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

/// 既定のズームレベル
pub const DEFAULT_ZOOM: u8 = 15;

/// 地図タイルの種別
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
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
}

impl MapParam {
    /// 使用するタイル種別
    pub fn tile_layer(&self) -> TileLayer {
        self.tile.unwrap_or_default()
    }

    /// タイルの最大ズームに丸めた実効ズームレベル
    pub fn effective_zoom(&self) -> u8 {
        let zoom = self.zoom.unwrap_or(DEFAULT_ZOOM);
        zoom.min(self.tile_layer().max_zoom()).max(1)
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
    fn test_tile_deserialized_from_value() {
        let p: MapParam = serde_json::from_str(r#"{"lat":35.0,"lon":139.0,"tile":"gsi"}"#)
            .expect("MapParamをデシリアライズできること");
        assert_eq!(p.tile_layer(), TileLayer::Gsi);
    }
}
