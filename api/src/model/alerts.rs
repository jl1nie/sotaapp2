use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use domain::model::common::activation::Alert;
use domain::model::AwardProgram;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTAAlert {
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

impl From<SOTAAlert> for Result<Alert> {
    fn from(a: SOTAAlert) -> Self {
        let date_activated = DateTime::parse_from_rfc3339(&a.date_activated)?;
        let date_activated = date_activated.with_timezone(&Utc);
        Ok(Alert {
            program: AwardProgram::SOTA,
            alert_id: a.id,
            user_id: a.user_id,
            reference: a.summit_code,
            reference_detail: a.summit_details,
            location: a.association_code,
            activator: a.activating_callsign,
            activator_name: None,
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
pub struct POTAAlert {
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

impl From<POTAAlert> for Result<Alert> {
    fn from(a: POTAAlert) -> Self {
        let tmformat = "%Y-%m-%d %H:%M";

        let start = a.start_date + " " + &a.start_time;
        let start = NaiveDateTime::parse_from_str(&start, tmformat)?;
        let start_time = Utc.from_local_datetime(&start).unwrap();

        let end = a.end_date + " " + &a.end_time;
        let end = NaiveDateTime::parse_from_str(&end, tmformat)?;
        let end_time = Utc.from_local_datetime(&end).unwrap();

        Ok(Alert {
            program: AwardProgram::POTA,
            alert_id: a.scheduled_activities_id,
            user_id: a.scheduler_user_id,
            reference: a.reference,
            reference_detail: a.name,
            location: a.location_desc,
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

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub program: String,
    pub alert_id: i32,
    pub user_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub location: String,
    pub activator: String,
    pub activator_name: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub frequencies: String,
    pub comment: Option<String>,
    pub poster: Option<String>,
}
impl From<Alert> for AlertResponse {
    fn from(a: Alert) -> Self {
        Self {
            program: a.program.into(),
            alert_id: a.alert_id,
            user_id: a.user_id,
            reference: a.reference,
            reference_detail: a.reference_detail,
            location: a.location,
            activator: a.activator,
            activator_name: a.activator_name,
            start_time: a.start_time,
            end_time: a.end_time,
            frequencies: a.frequencies,
            comment: a.comment,
            poster: a.poster,
        }
    }
}
