use chrono::NaiveDateTime;
use serde::Serialize;

use domain::model::aprslog::{AprsLog, AprsState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AprsLogView {
    pub time: NaiveDateTime,
    pub callsign: String,
    pub ssid: i32,
    pub destination: String,
    pub state: String,
    pub distance: f64,
    pub longitude: f64,
    pub latitude: f64,
}

impl From<AprsLog> for AprsLogView {
    fn from(l: AprsLog) -> AprsLogView {
        let AprsLog {
            callsign,
            destination,
            state,
            longitude,
            latitude,
        } = l;
        let (time, state, distance) = match state {
            AprsState::Approaching { time, distance } => (time, "Approaching", distance),
            AprsState::Climbing { time, distance } => (time, "Climbing", distance),
            AprsState::NearSummit { time, distance, .. } => (time, "NearSummit", distance),
            AprsState::OnSummit { time, distance, .. } => (time, "OnSummit", distance),
            AprsState::Descending { time, distance } => (time, "Descending", distance),
        };
        AprsLogView {
            time,
            callsign: callsign.callsign,
            ssid: callsign.ssid.unwrap_or_default() as i32,
            destination,
            state: state.to_string(),
            distance,
            longitude,
            latitude,
        }
    }
}
