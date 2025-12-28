use chrono::{DateTime, NaiveDate, Utc};
use domain::model::id::UserId;
use domain::model::Maidenhead;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use utoipa::ToSchema;

use common::utils::maidenhead;
use domain::model::event::PagenatedResult;
use domain::model::pota::{PotaLogHist, PotaLogStat, PotaLogStatEnt, PotaRefLog, PotaReference};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRefRequest {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: f64,
    pub latitude: f64,
}

impl From<CreateRefRequest> for Vec<PotaReference> {
    fn from(value: CreateRefRequest) -> Self {
        let CreateRefRequest {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_inactive,
            park_area,
            longitude,
            latitude,
        } = value;
        let update: DateTime<Utc> = Utc::now();
        vec![PotaReference {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_inactive,
            park_area,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            update,
        }]
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagenatedResponse<PotaReference> {
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
    pub results: Vec<PotaReference>,
}

impl From<PagenatedResult<PotaReference>> for PagenatedResponse<PotaRefView> {
    fn from(pagenated: PagenatedResult<PotaReference>) -> Self {
        PagenatedResponse {
            total: pagenated.total as i32,
            limit: pagenated.limit,
            offset: pagenated.offset,
            results: pagenated
                .results
                .into_iter()
                .map(PotaRefView::from)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: f64,
    pub latitude: f64,
}

impl From<UpdateRefRequest> for Vec<PotaReference> {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_inactive,
            park_area,
            longitude,
            latitude,
        } = value;
        let update: DateTime<Utc> = Utc::now();
        vec![PotaReference {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_inactive,
            park_area,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            update,
        }]
    }
}

#[derive(Debug)]
pub struct PotaActivatorLog {
    pub user_id: UserId,
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub attempts: i32,
    pub activations: i32,
    pub qsos: i32,
    pub upload: NaiveDate,
}

#[derive(Debug)]
pub struct PotaHunterLog {
    pub user_id: UserId,
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub qsos: i32,
    pub upload: NaiveDate,
}

/// POTAリファレンスビュー
#[derive(Debug, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct PotaRefView {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
}

impl From<PotaReference> for PotaRefView {
    fn from(pota: PotaReference) -> Self {
        PotaRefView {
            pota_code: pota.pota_code,
            wwff_code: pota.wwff_code,
            park_name: pota.park_name,
            park_name_j: pota.park_name_j,
            park_location: pota.park_location,
            park_locid: pota.park_locid,
            park_type: pota.park_type,
            park_inactive: pota.park_inactive,
            park_area: pota.park_area,
            longitude: pota.longitude,
            latitude: pota.latitude,
            maidenhead: pota.maidenhead,
        }
    }
}

/// POTAリファレンスログビュー
#[derive(Debug, Serialize, Default, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct PotaRefLogView {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
    pub attempts: Option<i32>,
    pub activations: Option<i32>,
    pub first_qso_date: Option<String>,
    pub qsos: Option<i32>,
}

impl From<PotaRefLog> for PotaRefLogView {
    fn from(pota: PotaRefLog) -> Self {
        PotaRefLogView {
            pota_code: pota.pota_code,
            wwff_code: pota.wwff_code,
            park_name: pota.park_name,
            park_name_j: pota.park_name_j,
            park_location: pota.park_location,
            park_locid: pota.park_locid,
            park_type: pota.park_type,
            park_inactive: pota.park_inactive,
            park_area: pota.park_area,
            longitude: pota.longitude,
            latitude: pota.latitude,
            maidenhead: pota.maidenhead,
            attempts: pota.attempts,
            activations: pota.activations,
            first_qso_date: pota.first_qso_date.map(|d| d.to_string()),
            qsos: pota.qsos,
        }
    }
}

/// POTA検索結果ビュー
#[derive(Debug, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct PotaSearchView {
    pub pota: String,
    pub wwff: String,
    pub name: String,
    pub name_j: String,
    pub locid: Vec<String>,
    pub area: i32,
    pub lon: f64,
    pub lat: f64,
    pub atmpt: Option<i32>,
    pub act: Option<i32>,
    pub date: Option<String>,
    pub qsos: Option<i32>,
}

impl From<PotaRefLog> for PotaSearchView {
    fn from(pota: PotaRefLog) -> Self {
        let locid: Vec<String> = pota
            .park_locid
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        PotaSearchView {
            pota: pota.pota_code,
            wwff: pota.wwff_code,
            name: pota.park_name,
            name_j: pota.park_name_j,
            locid,
            area: pota.park_area,
            lon: pota.longitude,
            lat: pota.latitude,
            atmpt: pota.attempts,
            act: pota.activations,
            date: pota.first_qso_date.map(|d| d.to_string()),
            qsos: pota.qsos,
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct PotaLogHistView {
    pub log_id: String,
    pub log_kind: String,
    pub last_update: String,
}

impl From<PotaLogHist> for PotaLogHistView {
    fn from(log: PotaLogHist) -> Self {
        let log_kind = match log.log_kind {
            Some(kind) => kind.into(),
            None => "none".to_string(),
        };

        PotaLogHistView {
            log_id: log.log_id.into(),
            log_kind,
            last_update: log.update.date().to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct PotaLogStatEntView {
    pub time: String,
    pub users: i32,
    pub logs: i32,
}

impl From<PotaLogStatEnt> for PotaLogStatEntView {
    fn from(stat: PotaLogStatEnt) -> Self {
        PotaLogStatEntView {
            time: stat.time,
            users: stat.users as i32,
            logs: stat.logs as i32,
        }
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct PotaLogStatView {
    pub log_uploaded: i32,
    pub log_entries: i32,
    pub log_expired: i32,
    pub log_error: i32,
    pub longest_id: String,
    pub longest_entry: i32,
    pub query_latency: String,
    pub log_history: Vec<PotaLogStatEntView>,
}

impl From<PotaLogStat> for PotaLogStatView {
    fn from(stat: PotaLogStat) -> Self {
        let query_latency = format!("{:.2}", stat.query_latency.as_secs_f32() * 1000f32);

        PotaLogStatView {
            log_uploaded: stat.log_uploaded as i32,
            log_entries: stat.log_entries as i32,
            log_expired: stat.log_expired as i32,
            log_error: stat.log_error as i32,
            longest_id: stat.longest_id.to_string(),
            longest_entry: stat.longest_entry as i32,
            query_latency,
            log_history: stat
                .log_history
                .into_iter()
                .map(PotaLogStatEntView::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use domain::model::pota::{PotaRefLog, PotaReference};

    fn create_test_pota_reference() -> PotaReference {
        PotaReference {
            pota_code: "JA-0001".to_string(),
            wwff_code: "JAFF-0001".to_string(),
            park_name: "Fuji-Hakone-Izu National Park".to_string(),
            park_name_j: "富士箱根伊豆国立公園".to_string(),
            park_location: "Shizuoka, Japan".to_string(),
            park_locid: "JP-22".to_string(),
            park_type: "National Park".to_string(),
            park_inactive: false,
            park_area: 121695,
            longitude: 138.7274,
            latitude: 35.3606,
            maidenhead: "PM95qi".to_string(),
            update: Utc::now(),
        }
    }

    fn create_test_pota_ref_log() -> PotaRefLog {
        PotaRefLog {
            pota_code: "JA-0001".to_string(),
            wwff_code: "JAFF-0001".to_string(),
            park_name: "Fuji-Hakone-Izu National Park".to_string(),
            park_name_j: "富士箱根伊豆国立公園".to_string(),
            park_location: "Shizuoka, Japan".to_string(),
            park_locid: "JP-22, JP-14".to_string(),
            park_type: "National Park".to_string(),
            park_inactive: false,
            park_area: 121695,
            longitude: 138.7274,
            latitude: 35.3606,
            maidenhead: "PM95qi".to_string(),
            attempts: Some(10),
            activations: Some(5),
            first_qso_date: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            qsos: Some(100),
        }
    }

    // =====================================================
    // PotaRefView 変換テスト
    // =====================================================

    #[test]
    fn test_pota_ref_view_from_reference() {
        let reference = create_test_pota_reference();
        let view: PotaRefView = reference.into();

        assert_eq!(view.pota_code, "JA-0001");
        assert_eq!(view.wwff_code, "JAFF-0001");
        assert_eq!(view.park_name, "Fuji-Hakone-Izu National Park");
        assert_eq!(view.park_name_j, "富士箱根伊豆国立公園");
        assert_eq!(view.park_location, "Shizuoka, Japan");
        assert_eq!(view.park_locid, "JP-22");
        assert_eq!(view.park_type, "National Park");
        assert!(!view.park_inactive);
        assert_eq!(view.park_area, 121695);
        assert!((view.longitude - 138.7274).abs() < 0.0001);
        assert!((view.latitude - 35.3606).abs() < 0.0001);
        assert_eq!(view.maidenhead, "PM95qi");
    }

    #[test]
    fn test_pota_ref_view_inactive_park() {
        let mut reference = create_test_pota_reference();
        reference.park_inactive = true;

        let view: PotaRefView = reference.into();

        assert!(view.park_inactive);
    }

    // =====================================================
    // PotaRefLogView 変換テスト
    // =====================================================

    #[test]
    fn test_pota_ref_log_view_from_ref_log() {
        let ref_log = create_test_pota_ref_log();
        let view: PotaRefLogView = ref_log.into();

        assert_eq!(view.pota_code, "JA-0001");
        assert_eq!(view.wwff_code, "JAFF-0001");
        assert_eq!(view.attempts, Some(10));
        assert_eq!(view.activations, Some(5));
        assert_eq!(view.first_qso_date, Some("2024-01-15".to_string()));
        assert_eq!(view.qsos, Some(100));
    }

    #[test]
    fn test_pota_ref_log_view_with_none_log_fields() {
        let mut ref_log = create_test_pota_ref_log();
        ref_log.attempts = None;
        ref_log.activations = None;
        ref_log.first_qso_date = None;
        ref_log.qsos = None;

        let view: PotaRefLogView = ref_log.into();

        assert!(view.attempts.is_none());
        assert!(view.activations.is_none());
        assert!(view.first_qso_date.is_none());
        assert!(view.qsos.is_none());
    }

    // =====================================================
    // PotaSearchView 変換テスト
    // =====================================================

    #[test]
    fn test_pota_search_view_from_ref_log() {
        let ref_log = create_test_pota_ref_log();
        let view: PotaSearchView = ref_log.into();

        assert_eq!(view.pota, "JA-0001");
        assert_eq!(view.wwff, "JAFF-0001");
        assert_eq!(view.name, "Fuji-Hakone-Izu National Park");
        assert_eq!(view.name_j, "富士箱根伊豆国立公園");
        // locidがカンマ区切りで分割される
        assert_eq!(view.locid, vec!["JP-22", "JP-14"]);
        assert_eq!(view.area, 121695);
        assert!((view.lon - 138.7274).abs() < 0.0001);
        assert!((view.lat - 35.3606).abs() < 0.0001);
        assert_eq!(view.atmpt, Some(10));
        assert_eq!(view.act, Some(5));
        assert_eq!(view.date, Some("2024-01-15".to_string()));
        assert_eq!(view.qsos, Some(100));
    }

    #[test]
    fn test_pota_search_view_single_locid() {
        let mut ref_log = create_test_pota_ref_log();
        ref_log.park_locid = "JP-22".to_string();

        let view: PotaSearchView = ref_log.into();

        assert_eq!(view.locid, vec!["JP-22"]);
    }

    // =====================================================
    // PagenatedResponse 変換テスト
    // =====================================================

    #[test]
    fn test_pota_pagenated_response_from_result() {
        let reference = create_test_pota_reference();
        let pagenated = PagenatedResult {
            total: 50,
            limit: 20,
            offset: 10,
            results: vec![reference],
        };

        let response: PagenatedResponse<PotaRefView> = pagenated.into();

        assert_eq!(response.total, 50);
        assert_eq!(response.limit, 20);
        assert_eq!(response.offset, 10);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].pota_code, "JA-0001");
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_pota_ref_view_json_serialization() {
        let reference = create_test_pota_reference();
        let view: PotaRefView = reference.into();

        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力されることを確認
        assert!(json.contains("potaCode"));
        assert!(json.contains("wwffCode"));
        assert!(json.contains("parkName"));
        assert!(json.contains("parkNameJ"));
        assert!(json.contains("parkLocation"));
        assert!(json.contains("parkLocid"));
        assert!(json.contains("parkType"));
        assert!(json.contains("parkInactive"));
        assert!(json.contains("parkArea"));
    }

    #[test]
    fn test_pota_search_view_json_serialization() {
        let ref_log = create_test_pota_ref_log();
        let view: PotaSearchView = ref_log.into();

        let json = serde_json::to_string(&view).unwrap();

        // 短縮フィールド名
        assert!(json.contains("\"pota\""));
        assert!(json.contains("\"wwff\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"nameJ\""));
        assert!(json.contains("\"locid\""));
        assert!(json.contains("\"area\""));
        assert!(json.contains("\"lon\""));
        assert!(json.contains("\"lat\""));
        assert!(json.contains("\"atmpt\""));
        assert!(json.contains("\"act\""));
        assert!(json.contains("\"date\""));
        assert!(json.contains("\"qsos\""));
    }
}
