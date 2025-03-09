use chrono::NaiveDateTime;
use serde::Serialize;

use domain::model::aprslog::{AprsLog, AprsState, AprsTrack};

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
            AprsState::Travelling { time } => (time, "Travelling", 0.0),
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
            destination: destination.unwrap_or_default(),
            state: state.to_string(),
            distance,
            longitude,
            latitude,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Tracks {
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Debug)]
pub struct Track {
    #[serde(rename = "type")]
    type_: String,
    geometry: Geometry,
    properties: Properties,
}

#[derive(Serialize, Debug)]
struct Geometry {
    #[serde(rename = "type")]
    type_: String,
    coordinates: Vec<Vec<f64>>,
}

#[derive(Serialize, Debug)]
struct Properties {
    callsign: String,
    ssid: Option<String>,
    lastseen: String,
    distance: i32,
    summit: String,
    spot_summit: Option<String>,
    spot_time: Option<String>,
    spot_freq: Option<String>,
    spot_mode: Option<String>,
    spot_comment: Option<String>,
}

impl From<AprsTrack> for Track {
    fn from(aprs: AprsTrack) -> Self {
        Track {
            type_: "Feature".to_string(),
            geometry: Geometry {
                type_: "LineString".to_string(),
                coordinates: aprs
                    .coordinates
                    .into_iter()
                    .map(|(lat, lon)| vec![lat, lon])
                    .collect(),
            },
            properties: Properties {
                callsign: aprs.callsign.callsign,
                ssid: aprs.callsign.ssid.map(|s| s.to_string()),
                lastseen: aprs.lastseen.to_rfc3339(),
                distance: aprs.distance.unwrap_or_default() as i32, // 距離を整数に変換しています
                summit: aprs.summit.unwrap_or_default(),
                spot_summit: aprs.spot_summit,
                spot_time: aprs.spot_time.map(|dt| dt.to_rfc3339()),
                spot_freq: aprs.spot_freq,
                spot_mode: aprs.spot_mode,
                spot_comment: aprs.spot_comment,
            },
        }
    }
}
