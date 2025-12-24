use serde::Serialize;
use typeshare::typeshare;

use super::pota::{PotaRefLogView, PotaSearchView};
use super::sota::{SotaRefView, SotaSearchView};
use domain::model::event::FindResult;

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub sota: Option<Vec<SotaSearchView>>,
    pub pota: Option<Vec<PotaSearchView>>,
}
impl From<FindResult> for SearchResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        Self {
            sota: if let Some(sota) = sota {
                let res = sota.into_iter().map(SotaSearchView::from).collect();
                Some(res)
            } else {
                None
            },
            pota: if let Some(pota) = pota {
                let res = pota.into_iter().map(PotaSearchView::from).collect();
                Some(res)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SearchFullResponse {
    pub sota: Option<Vec<SotaRefView>>,
    pub pota: Option<Vec<PotaRefLogView>>,
}
impl From<FindResult> for SearchFullResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        Self {
            sota: if let Some(sota) = sota {
                let res = sota.into_iter().map(SotaRefView::from).collect();
                Some(res)
            } else {
                None
            },
            pota: if let Some(pota) = pota {
                let res = pota.into_iter().map(PotaRefLogView::from).collect();
                Some(res)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SearchBriefResponse {
    pub count: u32,
    pub candidates: Vec<SearchBriefData>,
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SearchBriefData {
    pub code: String,
    pub lon: f64,
    pub lat: f64,
    pub name: String,
    pub name_j: String,
}

impl From<FindResult> for SearchBriefResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        let mut res = vec![];

        if let Some(sota) = sota {
            sota.iter().for_each(|r| {
                res.push(SearchBriefData {
                    code: r.summit_code.clone(),
                    lon: r.longitude,
                    lat: r.latitude,
                    name: r.summit_name.clone(),
                    name_j: r.summit_name_j.clone().unwrap_or_default(),
                })
            })
        };

        if let Some(pota) = pota {
            pota.into_iter().for_each(|r| {
                let code = match (r.pota_code.as_str(), r.wwff_code.as_str()) {
                    ("", wwff) => wwff.to_string(),
                    (pota, "") => pota.to_string(),
                    (pota, wwff) => format!("{}/{}", pota, wwff),
                };
                res.push(SearchBriefData {
                    code,
                    lon: r.longitude,
                    lat: r.latitude,
                    name: r.park_name,
                    name_j: r.park_name_j,
                })
            });
        };
        Self {
            count: res.len() as u32,
            candidates: res,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::model::pota::PotaRefLog;
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
            activation_date: None,
            activation_call: None,
        }
    }

    fn create_test_pota_ref_log() -> PotaRefLog {
        PotaRefLog {
            pota_code: "JA-0001".to_string(),
            wwff_code: "JAFF-0001".to_string(),
            park_name: "Ueno Park".to_string(),
            park_name_j: "上野公園".to_string(),
            park_location: "Tokyo".to_string(),
            park_locid: "JP-13".to_string(),
            park_type: "National Park".to_string(),
            park_inactive: false,
            park_area: 538506,
            longitude: 139.7730,
            latitude: 35.7126,
            maidenhead: "PM95sp".to_string(),
            first_qso_date: None,
            attempts: None,
            activations: None,
            qsos: None,
        }
    }

    // =====================================================
    // SearchResponse 変換テスト
    // =====================================================

    #[test]
    fn test_search_response_from_find_result_both() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: Some(vec![create_test_pota_ref_log()]),
        };

        let response: SearchResponse = result.into();

        assert!(response.sota.is_some());
        assert!(response.pota.is_some());
        assert_eq!(response.sota.as_ref().unwrap().len(), 1);
        assert_eq!(response.pota.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_search_response_from_find_result_sota_only() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: None,
        };

        let response: SearchResponse = result.into();

        assert!(response.sota.is_some());
        assert!(response.pota.is_none());
    }

    #[test]
    fn test_search_response_from_find_result_pota_only() {
        let result = FindResult {
            sota: None,
            pota: Some(vec![create_test_pota_ref_log()]),
        };

        let response: SearchResponse = result.into();

        assert!(response.sota.is_none());
        assert!(response.pota.is_some());
    }

    #[test]
    fn test_search_response_from_find_result_empty() {
        let result = FindResult {
            sota: None,
            pota: None,
        };

        let response: SearchResponse = result.into();

        assert!(response.sota.is_none());
        assert!(response.pota.is_none());
    }

    // =====================================================
    // SearchFullResponse 変換テスト
    // =====================================================

    #[test]
    fn test_search_full_response_from_find_result() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: Some(vec![create_test_pota_ref_log()]),
        };

        let response: SearchFullResponse = result.into();

        assert!(response.sota.is_some());
        assert!(response.pota.is_some());
        assert_eq!(response.sota.as_ref().unwrap()[0].summit_code, "JA/TK-001");
    }

    // =====================================================
    // SearchBriefResponse 変換テスト
    // =====================================================

    #[test]
    fn test_search_brief_response_from_find_result_both() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: Some(vec![create_test_pota_ref_log()]),
        };

        let response: SearchBriefResponse = result.into();

        assert_eq!(response.count, 2);
        assert_eq!(response.candidates.len(), 2);
    }

    #[test]
    fn test_search_brief_response_sota_data() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: None,
        };

        let response: SearchBriefResponse = result.into();

        assert_eq!(response.count, 1);
        assert_eq!(response.candidates[0].code, "JA/TK-001");
        assert_eq!(response.candidates[0].name, "Mt. Takao");
        assert_eq!(response.candidates[0].name_j, "高尾山");
        assert!((response.candidates[0].lon - 139.2438).abs() < 0.0001);
        assert!((response.candidates[0].lat - 35.6251).abs() < 0.0001);
    }

    #[test]
    fn test_search_brief_response_pota_data_both_codes() {
        let result = FindResult {
            sota: None,
            pota: Some(vec![create_test_pota_ref_log()]),
        };

        let response: SearchBriefResponse = result.into();

        assert_eq!(response.count, 1);
        // POTA code + WWFF code の両方がある場合は "JA-0001/JAFF-0001" 形式
        assert_eq!(response.candidates[0].code, "JA-0001/JAFF-0001");
        assert_eq!(response.candidates[0].name, "Ueno Park");
        assert_eq!(response.candidates[0].name_j, "上野公園");
    }

    #[test]
    fn test_search_brief_response_pota_data_pota_only() {
        let mut pota_ref = create_test_pota_ref_log();
        pota_ref.wwff_code = "".to_string();

        let result = FindResult {
            sota: None,
            pota: Some(vec![pota_ref]),
        };

        let response: SearchBriefResponse = result.into();

        // POTA codeのみの場合
        assert_eq!(response.candidates[0].code, "JA-0001");
    }

    #[test]
    fn test_search_brief_response_pota_data_wwff_only() {
        let mut pota_ref = create_test_pota_ref_log();
        pota_ref.pota_code = "".to_string();

        let result = FindResult {
            sota: None,
            pota: Some(vec![pota_ref]),
        };

        let response: SearchBriefResponse = result.into();

        // WWFF codeのみの場合
        assert_eq!(response.candidates[0].code, "JAFF-0001");
    }

    #[test]
    fn test_search_brief_response_empty() {
        let result = FindResult {
            sota: None,
            pota: None,
        };

        let response: SearchBriefResponse = result.into();

        assert_eq!(response.count, 0);
        assert!(response.candidates.is_empty());
    }

    #[test]
    fn test_search_brief_response_sota_none_name_j() {
        let mut sota = create_test_sota_reference();
        sota.summit_name_j = None;

        let result = FindResult {
            sota: Some(vec![sota]),
            pota: None,
        };

        let response: SearchBriefResponse = result.into();

        // summit_name_jがNoneの場合はデフォルトの空文字
        assert_eq!(response.candidates[0].name_j, "");
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_search_response_json_serialization() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: None,
        };

        let response: SearchResponse = result.into();
        let json = serde_json::to_string(&response).unwrap();

        // camelCase形式で出力される
        assert!(json.contains("\"sota\":[{"));
        assert!(json.contains("\"pota\":null"));
    }

    #[test]
    fn test_search_brief_response_json_serialization() {
        let result = FindResult {
            sota: Some(vec![create_test_sota_reference()]),
            pota: None,
        };

        let response: SearchBriefResponse = result.into();
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"count\":1"));
        assert!(json.contains("\"candidates\":[{"));
        assert!(json.contains("\"nameJ\":"));
    }
}
