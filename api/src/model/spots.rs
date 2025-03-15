use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use common::utils::call_to_operator;
use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "camelCase")]
pub struct SpotView {
    pub program: String,
    pub spot_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub activator: String,
    pub activator_name: Option<String>,
    pub spot_time: DateTime<Utc>,
    pub frequency: String,
    pub mode: String,
    pub spotter: String,
    pub comment: Option<String>,
    pub first_qso_date: Option<String>,
    pub qsos: Option<i32>,
}

impl From<SpotLog> for SpotView {
    fn from(s: SpotLog) -> Self {
        let first_qso_date = s.first_qso_date.map(|d| d.to_string());
        let qsos = s.qsos;
        let s = s.spot;
        Self {
            program: s.program.into(),
            spot_id: s.spot_id,
            reference: s.reference,
            reference_detail: s.reference_detail,
            activator: s.activator,
            activator_name: s.activator_name,
            spot_time: s.spot_time,
            frequency: s.frequency,
            mode: s.mode,
            spotter: s.spotter,
            comment: s.comment,
            first_qso_date,
            qsos,
        }
    }
}
