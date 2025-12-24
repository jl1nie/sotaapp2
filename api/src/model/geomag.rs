use serde::Serialize;
use typeshare::typeshare;

use domain::model::geomag::GeomagIndex;

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct GeomagView {
    pub date: String,
    pub a_index: i32,
    pub k_index: Vec<i32>,
}

impl From<GeomagIndex> for GeomagView {
    fn from(gi: GeomagIndex) -> GeomagView {
        let GeomagIndex {
            date,
            a_index,
            k_index,
        } = gi;
        GeomagView {
            date: date.to_string(),
            a_index,
            k_index: k_index.into_iter().map(|v| v as i32).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_test_geomag_index() -> GeomagIndex {
        GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            a_index: 12,
            k_index: vec![2.0, 3.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0],
        }
    }

    // =====================================================
    // GeomagView 変換テスト
    // =====================================================

    #[test]
    fn test_geomag_view_from_index() {
        let index = create_test_geomag_index();
        let view: GeomagView = index.into();

        assert_eq!(view.date, "2024-06-15");
        assert_eq!(view.a_index, 12);
        assert_eq!(view.k_index.len(), 8);
        assert_eq!(view.k_index[0], 2);
        assert_eq!(view.k_index[1], 3);
    }

    #[test]
    fn test_geomag_view_date_format() {
        let index = GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            a_index: 5,
            k_index: vec![],
        };

        let view: GeomagView = index.into();

        assert_eq!(view.date, "2024-01-01");
    }

    #[test]
    fn test_geomag_view_k_index_conversion() {
        // f32 -> i32 変換テスト
        let index = GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            a_index: 20,
            k_index: vec![1.5, 2.7, 3.1, 4.9],
        };

        let view: GeomagView = index.into();

        // f32からi32への変換は切り捨て
        assert_eq!(view.k_index[0], 1);
        assert_eq!(view.k_index[1], 2);
        assert_eq!(view.k_index[2], 3);
        assert_eq!(view.k_index[3], 4);
    }

    #[test]
    fn test_geomag_view_empty_k_index() {
        let index = GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            a_index: 0,
            k_index: vec![],
        };

        let view: GeomagView = index.into();

        assert!(view.k_index.is_empty());
        assert_eq!(view.a_index, 0);
    }

    #[test]
    fn test_geomag_view_high_a_index() {
        // 高い地磁気活動の場合
        let index = GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            a_index: 150,
            k_index: vec![7.0, 8.0, 9.0, 9.0, 8.0, 7.0, 6.0, 5.0],
        };

        let view: GeomagView = index.into();

        assert_eq!(view.a_index, 150);
        assert_eq!(view.k_index[2], 9);
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_geomag_view_json_serialization() {
        let index = create_test_geomag_index();
        let view: GeomagView = index.into();

        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力される
        assert!(json.contains("\"date\":\"2024-06-15\""));
        assert!(json.contains("\"aIndex\":12"));
        assert!(json.contains("\"kIndex\":[2,3,2,1,2,3,2,1]"));
    }

    #[test]
    fn test_geomag_view_json_field_names() {
        let index = GeomagIndex {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            a_index: 5,
            k_index: vec![1.0],
        };

        let view: GeomagView = index.into();
        let json = serde_json::to_string(&view).unwrap();

        // snake_caseではなくcamelCase
        assert!(json.contains("aIndex"));
        assert!(json.contains("kIndex"));
        assert!(!json.contains("a_index"));
        assert!(!json.contains("k_index"));
    }
}
