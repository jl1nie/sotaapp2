use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use utoipa::ToSchema;

use common::error::{AppError, AppResult};
use common::utils::call_to_operator;

use domain::model::activation::Alert;
use domain::model::AwardProgram;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SotaAlert {
    pub id: i32,
    #[serde(rename = "userID")]
    pub user_id: i32,
    pub time_stamp: String,
    pub date_activated: String,
    pub association_code: String,
    pub summit_code: String,
    pub summit_details: String,
    pub frequency: String,
    pub comments: Option<String>,
    pub activating_callsign: String,
    pub activator_name: String,
    pub poster_callsign: String,
    pub epoch: String,
}

impl From<SotaAlert> for AppResult<Alert> {
    fn from(a: SotaAlert) -> Self {
        let date_activated =
            DateTime::parse_from_rfc3339(&a.date_activated).map_err(AppError::ParseError)?;
        let date_activated = date_activated.with_timezone(&Utc);
        Ok(Alert {
            program: AwardProgram::SOTA,
            alert_id: a.id,
            user_id: a.user_id,
            reference: a.association_code.clone() + "/" + &a.summit_code,
            reference_detail: a.summit_details,
            location: a.association_code,
            activator_name: None,
            operator: call_to_operator(&a.activating_callsign),
            activator: a.activating_callsign,
            start_time: date_activated,
            end_time: None,
            frequencies: a.frequency,
            comment: a.comments,
            poster: Some(a.poster_callsign),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PotaAlert {
    pub scheduled_activities_id: i32,
    pub scheduler_user_id: i32,
    pub activator: String,
    pub name: String,
    pub reference: String,
    pub location_desc: String,
    pub activity_start: Option<String>,
    pub antivity_end: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub start_time: String,
    pub end_time: String,
    pub frequencies: String,
    pub comments: String,
}

impl From<PotaAlert> for AppResult<Alert> {
    fn from(a: PotaAlert) -> Self {
        let tmformat = "%Y-%m-%d %H:%M";

        let start = a.start_date + " " + &a.start_time;
        let start =
            NaiveDateTime::parse_from_str(&start, tmformat).map_err(AppError::ParseError)?;
        let start_time = Utc.from_local_datetime(&start).unwrap();

        let end = a.end_date + " " + &a.end_time;
        let end = NaiveDateTime::parse_from_str(&end, tmformat).map_err(AppError::ParseError)?;
        let end_time = Utc.from_local_datetime(&end).unwrap();

        Ok(Alert {
            program: AwardProgram::POTA,
            alert_id: a.scheduled_activities_id,
            user_id: a.scheduler_user_id,
            reference: a.reference,
            reference_detail: a.name,
            location: a.location_desc,
            operator: call_to_operator(&a.activator),
            activator: a.activator,
            activator_name: None,
            start_time,
            end_time: Some(end_time),
            frequencies: a.frequencies,
            comment: Some(a.comments),
            poster: None,
        })
    }
}

/// アラートビュー
#[derive(Debug, Serialize, ToSchema)]
#[typeshare]
pub struct AlertView {
    pub program: String,
    pub alert_id: i32,
    pub user_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub location: String,
    pub activator: String,
    pub operator: String,
    pub activator_name: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub frequencies: String,
    pub comment: Option<String>,
    pub poster: Option<String>,
}

impl From<Alert> for AlertView {
    fn from(a: Alert) -> Self {
        Self {
            program: a.program.into(),
            alert_id: a.alert_id,
            user_id: a.user_id,
            reference: a.reference,
            reference_detail: a.reference_detail,
            location: a.location,
            activator: a.activator,
            operator: a.operator,
            activator_name: a.activator_name,
            start_time: a.start_time.to_rfc3339(),
            end_time: a.end_time.map(|e| e.to_rfc3339()),
            frequencies: a.frequencies,
            comment: a.comment,
            poster: a.poster,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_test_alert(program: AwardProgram) -> Alert {
        Alert {
            program,
            alert_id: 12345,
            user_id: 100,
            reference: "JA/TK-001".to_string(),
            reference_detail: "Mt. Takao".to_string(),
            location: "Tokyo".to_string(),
            activator: "JA1ABC/P".to_string(),
            operator: "JA1ABC".to_string(),
            activator_name: Some("Taro Yamada".to_string()),
            start_time: Utc.with_ymd_and_hms(2024, 6, 15, 9, 0, 0).unwrap(),
            end_time: Some(Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap()),
            frequencies: "14.285 SSB".to_string(),
            comment: Some("Weather permitting".to_string()),
            poster: Some("JA2XYZ".to_string()),
        }
    }

    // =====================================================
    // AlertView 変換テスト
    // =====================================================

    #[test]
    fn test_alert_view_from_sota_alert() {
        let alert = create_test_alert(AwardProgram::SOTA);
        let view: AlertView = alert.into();

        assert_eq!(view.program, "SOTA");
        assert_eq!(view.alert_id, 12345);
        assert_eq!(view.user_id, 100);
        assert_eq!(view.reference, "JA/TK-001");
        assert_eq!(view.reference_detail, "Mt. Takao");
        assert_eq!(view.location, "Tokyo");
        assert_eq!(view.activator, "JA1ABC/P");
        assert_eq!(view.operator, "JA1ABC");
        assert_eq!(view.activator_name, Some("Taro Yamada".to_string()));
        assert_eq!(view.frequencies, "14.285 SSB");
        assert_eq!(view.comment, Some("Weather permitting".to_string()));
        assert_eq!(view.poster, Some("JA2XYZ".to_string()));
    }

    #[test]
    fn test_alert_view_from_pota_alert() {
        let alert = create_test_alert(AwardProgram::POTA);
        let view: AlertView = alert.into();

        assert_eq!(view.program, "POTA");
    }

    #[test]
    fn test_alert_view_start_time_format() {
        let alert = create_test_alert(AwardProgram::SOTA);
        let view: AlertView = alert.into();

        // RFC3339形式で出力される
        assert!(view.start_time.contains("2024-06-15"));
        assert!(view.start_time.contains("09:00:00"));
    }

    #[test]
    fn test_alert_view_end_time_some() {
        let alert = create_test_alert(AwardProgram::SOTA);
        let view: AlertView = alert.into();

        assert!(view.end_time.is_some());
        assert!(view.end_time.as_ref().unwrap().contains("2024-06-15"));
        assert!(view.end_time.as_ref().unwrap().contains("12:00:00"));
    }

    #[test]
    fn test_alert_view_end_time_none() {
        let mut alert = create_test_alert(AwardProgram::SOTA);
        alert.end_time = None;
        let view: AlertView = alert.into();

        assert!(view.end_time.is_none());
    }

    #[test]
    fn test_alert_view_optional_fields_none() {
        let mut alert = create_test_alert(AwardProgram::SOTA);
        alert.activator_name = None;
        alert.comment = None;
        alert.poster = None;

        let view: AlertView = alert.into();

        assert!(view.activator_name.is_none());
        assert!(view.comment.is_none());
        assert!(view.poster.is_none());
    }

    // =====================================================
    // SotaAlert デシリアライズテスト
    // =====================================================

    #[test]
    fn test_sota_alert_deserialize() {
        let json = r#"{
            "id": 1,
            "userID": 100,
            "timeStamp": "2024-06-15T00:00:00",
            "dateActivated": "2024-06-15T09:00:00+00:00",
            "associationCode": "JA",
            "summitCode": "TK-001",
            "summitDetails": "Mt. Takao",
            "frequency": "14.285 SSB",
            "comments": "QRV 9:00-12:00",
            "activatingCallsign": "JA1ABC/P",
            "activatorName": "Taro",
            "posterCallsign": "JA2XYZ",
            "epoch": "1718442000"
        }"#;

        let alert: SotaAlert = serde_json::from_str(json).unwrap();

        assert_eq!(alert.id, 1);
        assert_eq!(alert.user_id, 100);
        assert_eq!(alert.association_code, "JA");
        assert_eq!(alert.summit_code, "TK-001");
        assert_eq!(alert.activating_callsign, "JA1ABC/P");
    }

    #[test]
    fn test_sota_alert_to_domain_alert() {
        let json = r#"{
            "id": 1,
            "userID": 100,
            "timeStamp": "2024-06-15T00:00:00",
            "dateActivated": "2024-06-15T09:00:00+00:00",
            "associationCode": "JA",
            "summitCode": "TK-001",
            "summitDetails": "Mt. Takao",
            "frequency": "14.285",
            "comments": null,
            "activatingCallsign": "JA1ABC",
            "activatorName": "Taro",
            "posterCallsign": "JA2XYZ",
            "epoch": "1718442000"
        }"#;

        let sota_alert: SotaAlert = serde_json::from_str(json).unwrap();
        let alert: AppResult<Alert> = sota_alert.into();
        let alert = alert.unwrap();

        assert!(matches!(alert.program, AwardProgram::SOTA));
        assert_eq!(alert.reference, "JA/TK-001");
        assert_eq!(alert.location, "JA");
        assert!(alert.comment.is_none());
    }

    // =====================================================
    // PotaAlert デシリアライズテスト
    // =====================================================

    #[test]
    fn test_pota_alert_deserialize() {
        let json = r#"{
            "scheduledActivitiesId": 5000,
            "schedulerUserId": 200,
            "activator": "JA1XYZ/P",
            "name": "Ueno Park",
            "reference": "JA-0001",
            "locationDesc": "Tokyo",
            "activityStart": null,
            "antivityEnd": null,
            "startDate": "2024-06-15",
            "endDate": "2024-06-15",
            "startTime": "09:00",
            "endTime": "12:00",
            "frequencies": "7.144 SSB",
            "comments": "Portable operation"
        }"#;

        let alert: PotaAlert = serde_json::from_str(json).unwrap();

        assert_eq!(alert.scheduled_activities_id, 5000);
        assert_eq!(alert.scheduler_user_id, 200);
        assert_eq!(alert.reference, "JA-0001");
        assert_eq!(alert.activator, "JA1XYZ/P");
    }

    #[test]
    fn test_pota_alert_to_domain_alert() {
        let json = r#"{
            "scheduledActivitiesId": 5000,
            "schedulerUserId": 200,
            "activator": "JA1XYZ",
            "name": "Ueno Park",
            "reference": "JA-0001",
            "locationDesc": "Tokyo",
            "activityStart": null,
            "antivityEnd": null,
            "startDate": "2024-06-15",
            "endDate": "2024-06-15",
            "startTime": "09:00",
            "endTime": "12:00",
            "frequencies": "7.144 SSB",
            "comments": "Portable operation"
        }"#;

        let pota_alert: PotaAlert = serde_json::from_str(json).unwrap();
        let alert: AppResult<Alert> = pota_alert.into();
        let alert = alert.unwrap();

        assert!(matches!(alert.program, AwardProgram::POTA));
        assert_eq!(alert.reference, "JA-0001");
        assert!(alert.end_time.is_some());
        assert!(alert.poster.is_none());
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_alert_view_json_serialization() {
        let alert = create_test_alert(AwardProgram::SOTA);
        let view: AlertView = alert.into();

        let json = serde_json::to_string(&view).unwrap();

        assert!(json.contains("\"program\":\"SOTA\""));
        assert!(json.contains("\"alert_id\":12345"));
        assert!(json.contains("\"reference\":\"JA/TK-001\""));
        assert!(json.contains("\"activator\":\"JA1ABC/P\""));
    }
}
