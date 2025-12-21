use chrono::NaiveDate;
use csv::ReaderBuilder;
use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use maidenhead::longlat_to_grid;
use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

pub fn csv_reader<T: DeserializeOwned + std::fmt::Debug>(
    csv: String,
    has_headers: bool,
    skip: usize,
) -> AppResult<Vec<T>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .flexible(true)
        .from_reader(csv.as_bytes());

    let mut reflist: Vec<T> = Vec::new();
    for result in rdr.records().skip(skip) {
        let req: T = result
            .map_err(AppError::CSVReadError)?
            .deserialize(None)
            .map_err(AppError::CSVReadError)?;
        reflist.push(req);
    }
    Ok(reflist)
}

pub fn call_to_operator(callsign: &str) -> String {
    let callsign = callsign.trim_end().to_string();
    let parts: Vec<_> = callsign.split("/").collect();
    match parts.len() {
        1 => parts[0].to_string(),
        2 => parts[0].to_string(),
        3 => parts[1].to_string(),
        _ => callsign,
    }
}

pub fn calculate_bounding_box(lat: f64, lon: f64, distance: f64) -> (f64, f64, f64, f64) {
    let g = Geodesic::wgs84();

    let (max_lat, max_lon, _, _) = g.direct(lat, lon, 45.0, distance);
    let (min_lat, min_lon, _, _) = g.direct(lat, lon, 225.0, distance);

    (min_lat, min_lon, max_lat, max_lon)
}

pub fn calculate_distance(lat: f64, lon: f64, lat2: f64, lon2: f64) -> f64 {
    let g = Geodesic::wgs84();

    g.inverse(lat, lon, lat2, lon2)
}

pub fn maidenhead(lon: f64, lat: f64) -> String {
    let lon = if lon >= 180.0 { lon - 360.0f64 } else { lon };
    longlat_to_grid(lon, lat, 8).unwrap_or("--------".to_string())
}

/// 複数フォーマットに対応した日付パース
/// フォーマット: "YYYY-MM-DD", "YYYY/MM/DD", "DD/MM/YYYY"
pub fn parse_date_flexible(date_str: &str) -> Option<NaiveDate> {
    const FORMATS: &[&str] = &["%Y-%m-%d", "%Y/%m/%d", "%d/%m/%Y"];
    for fmt in FORMATS {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, fmt) {
            return Some(date);
        }
    }
    None
}

/// 日付パース（デフォルト値つき）
pub fn parse_date_or_default(date_str: &str, default: NaiveDate) -> NaiveDate {
    parse_date_flexible(date_str).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_flexible_iso() {
        let date = parse_date_flexible("2025-06-01").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    }

    #[test]
    fn test_parse_date_flexible_slash() {
        let date = parse_date_flexible("2025/06/01").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    }

    #[test]
    fn test_parse_date_flexible_dmy() {
        let date = parse_date_flexible("01/06/2025").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    }

    #[test]
    fn test_parse_date_flexible_invalid() {
        assert!(parse_date_flexible("invalid").is_none());
        assert!(parse_date_flexible("").is_none());
    }

    #[test]
    fn test_call_to_operator_simple() {
        assert_eq!(call_to_operator("JA1ABC"), "JA1ABC");
    }

    #[test]
    fn test_call_to_operator_with_suffix() {
        assert_eq!(call_to_operator("JA1ABC/P"), "JA1ABC");
    }

    #[test]
    fn test_call_to_operator_with_prefix_and_suffix() {
        assert_eq!(call_to_operator("JA0/JA1ABC/P"), "JA1ABC");
    }

    #[test]
    fn test_maidenhead_tokyo() {
        // 東京駅付近: 139.7671, 35.6812
        let grid = maidenhead(139.7671, 35.6812);
        assert!(grid.starts_with("PM95"));
    }

    #[test]
    fn test_maidenhead_over_180() {
        // 経度180度以上の場合の正規化
        let grid = maidenhead(190.0, 35.0);
        assert!(!grid.contains('-')); // 正規化されてグリッドが取得できること
    }

    #[test]
    fn test_calculate_distance_same_point() {
        let dist = calculate_distance(35.0, 139.0, 35.0, 139.0);
        assert!(dist < 1.0); // 同一点なので距離はほぼ0
    }

    #[test]
    fn test_calculate_distance_tokyo_osaka() {
        // 東京-大阪間: 約400km
        let dist = calculate_distance(35.6812, 139.7671, 34.6937, 135.5023);
        assert!(dist > 350_000.0 && dist < 450_000.0);
    }

    #[test]
    fn test_calculate_bounding_box() {
        let (min_lat, min_lon, max_lat, max_lon) = calculate_bounding_box(35.0, 139.0, 10000.0);
        // 10km範囲のバウンディングボックス
        assert!(min_lat < 35.0 && max_lat > 35.0);
        assert!(min_lon < 139.0 && max_lon > 139.0);
    }
}
