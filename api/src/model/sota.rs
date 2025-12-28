use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use utoipa::ToSchema;

use common::utils::maidenhead;
use domain::model::event::PagenatedResult;
use domain::model::sota::SotaReference;
use domain::model::Maidenhead;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateRefRequest {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<CreateRefRequest> for Vec<SotaReference> {
    fn from(value: CreateRefRequest) -> Self {
        let CreateRefRequest {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;
        vec![SotaReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j: Some(summit_name_j),
            city: Some(city),
            city_j: Some(city_j),
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from: NaiveDate::parse_from_str(&valid_from, "%d/%m/%Y").unwrap(),
            valid_to: NaiveDate::parse_from_str(&valid_to, "%d/%m/%Y").unwrap(),
            activation_count,
            activation_date,
            activation_call,
        }]
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<UpdateRefRequest> for Vec<SotaReference> {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;
        let request = SotaReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j: Some(summit_name_j),
            city: Some(city),
            city_j: Some(city_j),
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from: NaiveDate::parse_from_str(&valid_from, "%d/%m/%Y").unwrap(),
            valid_to: NaiveDate::parse_from_str(&valid_to, "%d/%m/%Y").unwrap(),
            activation_count,
            activation_date,
            activation_call,
        };
        vec![request]
    }
}

/// SOTAリファレンス詳細ビュー
#[derive(Debug, Serialize, Default, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SotaRefView {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
    pub points: i32,
    pub bonus_points: i32,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SotaReference> for SotaRefView {
    #[allow(unused_variables)]
    fn from(value: SotaReference) -> Self {
        let SotaReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;

        Self {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

impl From<(Maidenhead, SotaReference)> for SotaRefView {
    #[allow(unused_variables)]
    fn from((maidenhead, value): (Maidenhead, SotaReference)) -> Self {
        let SotaReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;

        Self {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct PagenatedResponse<SOTAReference> {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub results: Vec<SOTAReference>,
}

impl From<PagenatedResult<SotaReference>> for PagenatedResponse<SotaRefView> {
    fn from(pagenated: PagenatedResult<SotaReference>) -> Self {
        PagenatedResponse {
            total: pagenated.total as i32,
            limit: pagenated.limit,
            offset: pagenated.offset,
            results: pagenated
                .results
                .into_iter()
                .map(SotaRefView::from)
                .collect(),
        }
    }
}

/// SOTA検索結果ビュー
#[derive(Debug, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SotaSearchView {
    pub code: String,
    pub name: String,
    pub name_j: Option<String>,
    pub alt: i32,
    pub lon: f64,
    pub lat: f64,
    pub pts: i32,
    pub count: i32,
}

impl From<SotaReference> for SotaSearchView {
    fn from(value: SotaReference) -> Self {
        let SotaReference {
            summit_code,
            summit_name,
            summit_name_j,
            alt_m,
            longitude,
            latitude,
            points,
            activation_count,
            ..
        } = value;

        Self {
            code: summit_code,
            name: summit_name,
            name_j: summit_name_j,
            alt: alt_m,
            lon: longitude,
            lat: latitude,
            pts: points,
            count: activation_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::model::sota::SotaReference;

    fn create_test_sota_reference() -> SotaReference {
        SotaReference {
            summit_code: "JA/TK-001".to_string(),
            association_name: "Japan".to_string(),
            region_name: "Tokyo".to_string(),
            summit_name: "Mt. Takao".to_string(),
            summit_name_j: Some("高尾山".to_string()),
            city: Some("Hachioji".to_string()),
            city_j: Some("八王子市".to_string()),
            alt_m: 599,
            alt_ft: 1965,
            grid_ref1: "".to_string(),
            grid_ref2: "".to_string(),
            longitude: 139.2438,
            latitude: 35.6251,
            maidenhead: "PM95po".to_string(),
            points: 4,
            bonus_points: 0,
            valid_from: NaiveDate::from_ymd_opt(2010, 1, 1).unwrap(),
            valid_to: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            activation_count: 500,
            activation_date: Some("2024-01-01".to_string()),
            activation_call: Some("JA1ABC".to_string()),
        }
    }

    // =====================================================
    // SotaRefView 変換テスト
    // =====================================================

    #[test]
    fn test_sota_ref_view_from_reference() {
        let reference = create_test_sota_reference();
        let view: SotaRefView = reference.into();

        assert_eq!(view.summit_code, "JA/TK-001");
        assert_eq!(view.association_name, "Japan");
        assert_eq!(view.region_name, "Tokyo");
        assert_eq!(view.summit_name, "Mt. Takao");
        assert_eq!(view.summit_name_j, Some("高尾山".to_string()));
        assert_eq!(view.city, Some("Hachioji".to_string()));
        assert_eq!(view.city_j, Some("八王子市".to_string()));
        assert_eq!(view.alt_m, 599);
        assert!((view.longitude - 139.2438).abs() < 0.0001);
        assert!((view.latitude - 35.6251).abs() < 0.0001);
        assert_eq!(view.maidenhead, "PM95po");
        assert_eq!(view.points, 4);
        assert_eq!(view.bonus_points, 0);
        assert_eq!(view.activation_count, 500);
        assert_eq!(view.activation_date, Some("2024-01-01".to_string()));
        assert_eq!(view.activation_call, Some("JA1ABC".to_string()));
    }

    #[test]
    fn test_sota_ref_view_with_none_optional_fields() {
        let mut reference = create_test_sota_reference();
        reference.summit_name_j = None;
        reference.city = None;
        reference.city_j = None;
        reference.activation_date = None;
        reference.activation_call = None;

        let view: SotaRefView = reference.into();

        assert!(view.summit_name_j.is_none());
        assert!(view.city.is_none());
        assert!(view.city_j.is_none());
        assert!(view.activation_date.is_none());
        assert!(view.activation_call.is_none());
    }

    // =====================================================
    // SotaSearchView 変換テスト
    // =====================================================

    #[test]
    fn test_sota_search_view_from_reference() {
        let reference = create_test_sota_reference();
        let view: SotaSearchView = reference.into();

        assert_eq!(view.code, "JA/TK-001");
        assert_eq!(view.name, "Mt. Takao");
        assert_eq!(view.name_j, Some("高尾山".to_string()));
        assert_eq!(view.alt, 599);
        assert!((view.lon - 139.2438).abs() < 0.0001);
        assert!((view.lat - 35.6251).abs() < 0.0001);
        assert_eq!(view.pts, 4);
        assert_eq!(view.count, 500);
    }

    // =====================================================
    // PagenatedResponse 変換テスト
    // =====================================================

    #[test]
    fn test_pagenated_response_from_result() {
        let reference = create_test_sota_reference();
        let pagenated = PagenatedResult {
            total: 100,
            limit: 10,
            offset: 20,
            results: vec![reference],
        };

        let response: PagenatedResponse<SotaRefView> = pagenated.into();

        assert_eq!(response.total, 100);
        assert_eq!(response.limit, 10);
        assert_eq!(response.offset, 20);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].summit_code, "JA/TK-001");
    }

    #[test]
    fn test_pagenated_response_empty_results() {
        let pagenated: PagenatedResult<SotaReference> = PagenatedResult {
            total: 0,
            limit: 10,
            offset: 0,
            results: vec![],
        };

        let response: PagenatedResponse<SotaRefView> = pagenated.into();

        assert_eq!(response.total, 0);
        assert!(response.results.is_empty());
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_sota_ref_view_json_serialization() {
        let reference = create_test_sota_reference();
        let view: SotaRefView = reference.into();

        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力されることを確認
        assert!(json.contains("summitCode"));
        assert!(json.contains("associationName"));
        assert!(json.contains("regionName"));
        assert!(json.contains("summitName"));
        assert!(json.contains("summitNameJ"));
        assert!(json.contains("altM"));
        assert!(json.contains("activationCount"));
    }

    #[test]
    fn test_sota_search_view_json_serialization() {
        let reference = create_test_sota_reference();
        let view: SotaSearchView = reference.into();

        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で短縮フィールド名
        assert!(json.contains("\"code\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"nameJ\""));
        assert!(json.contains("\"alt\""));
        assert!(json.contains("\"lon\""));
        assert!(json.contains("\"lat\""));
        assert!(json.contains("\"pts\""));
        assert!(json.contains("\"count\""));
    }
}
