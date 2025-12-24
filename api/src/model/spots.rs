use chrono::{NaiveDateTime, TimeZone, Utc};
use common::utils::call_to_operator;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use common::error::{AppError, AppResult};
use domain::model::activation::{Spot, SpotLog};
use domain::model::AwardProgram;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SotaSpot {
    pub id: i32,
    #[serde(rename = "userID")]
    pub user_id: i32,
    pub time_stamp: String,
    pub comments: Option<String>,
    pub callsign: String,
    pub association_code: String,
    pub summit_code: String,
    pub activator_callsign: String,
    pub activator_name: String,
    pub frequency: String,
    pub mode: String,
    pub summit_details: String,
    pub highlight_color: Option<String>,
}

impl From<SotaSpot> for AppResult<Spot> {
    fn from(s: SotaSpot) -> Self {
        let naive = NaiveDateTime::parse_from_str(&s.time_stamp, "%Y-%m-%dT%H:%M:%S")
            .map_err(AppError::ParseError)?;
        let spot_time = Utc.from_local_datetime(&naive).unwrap();
        Ok(Spot {
            program: AwardProgram::SOTA,
            spot_id: s.id,
            reference: s.association_code + "/" + &s.summit_code,
            reference_detail: s.summit_details,
            operator: call_to_operator(&s.activator_callsign),
            activator: s.activator_callsign,
            activator_name: Some(s.activator_name),
            spot_time,
            frequency: s.frequency,
            mode: s.mode,
            spotter: s.callsign,
            comment: s.comments,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PotaSpot {
    pub spot_id: i32,
    pub activator: String,
    pub frequency: String,
    pub mode: String,
    pub reference: String,
    pub park_name: Option<String>,
    pub spot_time: String,
    pub spotter: String,
    pub comments: Option<String>,
    pub source: String,
    pub invalid: Option<String>,
    pub name: String,
    pub location_desc: String,
    pub grid4: String,
    pub grid6: String,
    pub latitude: f64,
    pub longitude: f64,
    pub count: i32,
    pub expire: i32,
}

impl From<PotaSpot> for AppResult<Spot> {
    fn from(s: PotaSpot) -> Self {
        let naive = NaiveDateTime::parse_from_str(&s.spot_time, "%Y-%m-%dT%H:%M:%S")
            .map_err(AppError::ParseError)?;
        let spot_time = Utc.from_local_datetime(&naive).unwrap();
        Ok(Spot {
            program: AwardProgram::POTA,
            spot_id: s.spot_id,
            reference: s.reference,
            reference_detail: s.name,
            operator: call_to_operator(&s.activator),
            activator: s.activator,
            activator_name: None,
            spot_time,
            frequency: s.frequency,
            mode: s.mode,
            spotter: s.spotter,
            comment: s.comments,
        })
    }
}

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct SpotView {
    pub program: String,
    pub spot_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub activator: String,
    pub activator_name: Option<String>,
    pub spot_time: String,
    pub frequency: String,
    pub mode: String,
    pub spotter: String,
    pub comment: Option<String>,
    pub qsos: Option<i32>,
}

impl From<SpotLog> for SpotView {
    fn from(s: SpotLog) -> Self {
        let qsos = s.qsos;
        let s = s.spot;
        Self {
            program: s.program.into(),
            spot_id: s.spot_id,
            reference: s.reference,
            reference_detail: s.reference_detail,
            activator: s.activator,
            activator_name: s.activator_name,
            spot_time: s.spot_time.to_rfc3339(),
            frequency: s.frequency,
            mode: s.mode,
            spotter: s.spotter,
            comment: s.comment,
            qsos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_test_spot(program: AwardProgram) -> Spot {
        Spot {
            program,
            spot_id: 99999,
            reference: "JA/TK-001".to_string(),
            reference_detail: "Mt. Takao".to_string(),
            operator: "JA1ABC".to_string(),
            activator: "JA1ABC/P".to_string(),
            activator_name: Some("Taro Yamada".to_string()),
            spot_time: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            frequency: "14.285".to_string(),
            mode: "SSB".to_string(),
            spotter: "JA2XYZ".to_string(),
            comment: Some("Good signal".to_string()),
        }
    }

    fn create_test_spot_log(program: AwardProgram, qsos: Option<i32>) -> SpotLog {
        SpotLog {
            spot: create_test_spot(program),
            qsos,
        }
    }

    // =====================================================
    // SpotView 変換テスト
    // =====================================================

    #[test]
    fn test_spot_view_from_spot_log() {
        let spot_log = create_test_spot_log(AwardProgram::SOTA, Some(15));
        let view: SpotView = spot_log.into();

        assert_eq!(view.program, "SOTA");
        assert_eq!(view.spot_id, 99999);
        assert_eq!(view.reference, "JA/TK-001");
        assert_eq!(view.reference_detail, "Mt. Takao");
        assert_eq!(view.activator, "JA1ABC/P");
        assert_eq!(view.activator_name, Some("Taro Yamada".to_string()));
        assert_eq!(view.frequency, "14.285");
        assert_eq!(view.mode, "SSB");
        assert_eq!(view.spotter, "JA2XYZ");
        assert_eq!(view.comment, Some("Good signal".to_string()));
        assert_eq!(view.qsos, Some(15));
    }

    #[test]
    fn test_spot_view_from_pota_spot_log() {
        let spot_log = create_test_spot_log(AwardProgram::POTA, Some(25));
        let view: SpotView = spot_log.into();

        assert_eq!(view.program, "POTA");
        assert_eq!(view.qsos, Some(25));
    }

    #[test]
    fn test_spot_view_qsos_none() {
        let spot_log = create_test_spot_log(AwardProgram::SOTA, None);
        let view: SpotView = spot_log.into();

        assert!(view.qsos.is_none());
    }

    #[test]
    fn test_spot_view_spot_time_format() {
        let spot_log = create_test_spot_log(AwardProgram::SOTA, None);
        let view: SpotView = spot_log.into();

        // RFC3339形式で出力される
        assert!(view.spot_time.contains("2024-06-15"));
        assert!(view.spot_time.contains("10:30:00"));
    }

    #[test]
    fn test_spot_view_optional_fields_none() {
        let mut spot_log = create_test_spot_log(AwardProgram::SOTA, None);
        spot_log.spot.activator_name = None;
        spot_log.spot.comment = None;

        let view: SpotView = spot_log.into();

        assert!(view.activator_name.is_none());
        assert!(view.comment.is_none());
    }

    // =====================================================
    // SotaSpot デシリアライズテスト
    // =====================================================

    #[test]
    fn test_sota_spot_deserialize() {
        let json = r#"{
            "id": 123456,
            "userID": 100,
            "timeStamp": "2024-06-15T10:30:00",
            "comments": "S9+",
            "callsign": "JA2XYZ",
            "associationCode": "JA",
            "summitCode": "TK-001",
            "activatorCallsign": "JA1ABC/P",
            "activatorName": "Taro",
            "frequency": "14.285",
            "mode": "SSB",
            "summitDetails": "Mt. Takao",
            "highlightColor": null
        }"#;

        let spot: SotaSpot = serde_json::from_str(json).unwrap();

        assert_eq!(spot.id, 123456);
        assert_eq!(spot.user_id, 100);
        assert_eq!(spot.callsign, "JA2XYZ");
        assert_eq!(spot.association_code, "JA");
        assert_eq!(spot.summit_code, "TK-001");
        assert_eq!(spot.activator_callsign, "JA1ABC/P");
        assert_eq!(spot.frequency, "14.285");
        assert_eq!(spot.mode, "SSB");
    }

    #[test]
    fn test_sota_spot_to_domain_spot() {
        let json = r#"{
            "id": 123456,
            "userID": 100,
            "timeStamp": "2024-06-15T10:30:00",
            "comments": null,
            "callsign": "JA2XYZ",
            "associationCode": "JA",
            "summitCode": "TK-001",
            "activatorCallsign": "JA1ABC",
            "activatorName": "Taro",
            "frequency": "14.285",
            "mode": "CW",
            "summitDetails": "Mt. Takao",
            "highlightColor": null
        }"#;

        let sota_spot: SotaSpot = serde_json::from_str(json).unwrap();
        let spot: AppResult<Spot> = sota_spot.into();
        let spot = spot.unwrap();

        assert!(matches!(spot.program, AwardProgram::SOTA));
        assert_eq!(spot.reference, "JA/TK-001");
        assert_eq!(spot.spotter, "JA2XYZ");
        assert!(spot.comment.is_none());
    }

    // =====================================================
    // PotaSpot デシリアライズテスト
    // =====================================================

    #[test]
    fn test_pota_spot_deserialize() {
        let json = r#"{
            "spotId": 789012,
            "activator": "JA1XYZ/P",
            "frequency": "7.144",
            "mode": "SSB",
            "reference": "JA-0001",
            "parkName": "Ueno Park",
            "spotTime": "2024-06-15T11:00:00",
            "spotter": "JA3ABC",
            "comments": "Strong signal",
            "source": "POTA",
            "invalid": null,
            "name": "Ueno Onshi Park",
            "locationDesc": "Tokyo",
            "grid4": "PM95",
            "grid6": "PM95po",
            "latitude": 35.7126,
            "longitude": 139.7730,
            "count": 10,
            "expire": 3600
        }"#;

        let spot: PotaSpot = serde_json::from_str(json).unwrap();

        assert_eq!(spot.spot_id, 789012);
        assert_eq!(spot.activator, "JA1XYZ/P");
        assert_eq!(spot.reference, "JA-0001");
        assert_eq!(spot.spotter, "JA3ABC");
        assert_eq!(spot.grid6, "PM95po");
    }

    #[test]
    fn test_pota_spot_to_domain_spot() {
        let json = r#"{
            "spotId": 789012,
            "activator": "JA1XYZ",
            "frequency": "7.144",
            "mode": "FT8",
            "reference": "JA-0001",
            "parkName": null,
            "spotTime": "2024-06-15T11:00:00",
            "spotter": "JA3ABC",
            "comments": null,
            "source": "RBN",
            "invalid": null,
            "name": "Ueno Park",
            "locationDesc": "Tokyo",
            "grid4": "PM95",
            "grid6": "PM95po",
            "latitude": 35.7126,
            "longitude": 139.7730,
            "count": 5,
            "expire": 1800
        }"#;

        let pota_spot: PotaSpot = serde_json::from_str(json).unwrap();
        let spot: AppResult<Spot> = pota_spot.into();
        let spot = spot.unwrap();

        assert!(matches!(spot.program, AwardProgram::POTA));
        assert_eq!(spot.reference, "JA-0001");
        assert_eq!(spot.reference_detail, "Ueno Park");
        assert!(spot.activator_name.is_none());
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_spot_view_json_serialization() {
        let spot_log = create_test_spot_log(AwardProgram::SOTA, Some(20));
        let view: SpotView = spot_log.into();

        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力される
        assert!(json.contains("\"program\":\"SOTA\""));
        assert!(json.contains("\"spotId\":99999"));
        assert!(json.contains("\"referenceDetail\":\"Mt. Takao\""));
        assert!(json.contains("\"activatorName\":\"Taro Yamada\""));
        assert!(json.contains("\"spotTime\":"));
        assert!(json.contains("\"qsos\":20"));
    }

    #[test]
    fn test_spot_view_json_null_qsos() {
        let spot_log = create_test_spot_log(AwardProgram::POTA, None);
        let view: SpotView = spot_log.into();

        let json = serde_json::to_string(&view).unwrap();

        assert!(json.contains("\"qsos\":null"));
    }
}
