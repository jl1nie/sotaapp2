use aprs_message::AprsCallsign;
use chrono::NaiveDateTime;
use domain::model::aprslog::{AprsLog, AprsState};

#[derive(Debug, sqlx::FromRow)]
pub struct AprsLogRow {
    pub time: NaiveDateTime,
    pub callsign: String,
    pub ssid: i64,
    pub destination: Option<String>,
    pub distance: Option<f64>,
    pub state: i64,
    pub message: Option<String>,
    pub longitude: f64,
    pub latitude: f64,
}

impl From<AprsLogRow> for AprsLog {
    fn from(aprs_log: AprsLogRow) -> Self {
        let state = match aprs_log.state {
            0 => AprsState::Travelling {
                time: aprs_log.time,
            },
            1 => AprsState::Approaching {
                time: aprs_log.time,
                distance: aprs_log.distance.unwrap_or_default(),
            },
            2 => AprsState::Climbing {
                time: aprs_log.time,
                distance: aprs_log.distance.unwrap_or_default(),
            },
            3 => AprsState::NearSummit {
                time: aprs_log.time,
                distance: aprs_log.distance.unwrap_or_default(),
                message: aprs_log.message.unwrap_or_default(),
            },
            4 => AprsState::OnSummit {
                time: aprs_log.time,
                distance: aprs_log.distance.unwrap_or_default(),
                message: aprs_log.message.unwrap_or_default(),
            },
            5 => AprsState::Descending {
                time: aprs_log.time,
                distance: aprs_log.distance.unwrap_or_default(),
            },
            _ => panic!("Invalid state"),
        };
        let ssid = if aprs_log.ssid == 0 {
            None
        } else {
            Some(aprs_log.ssid as u32)
        };
        AprsLog {
            callsign: AprsCallsign {
                callsign: aprs_log.callsign,
                ssid,
            },
            destination: aprs_log.destination,
            state,
            longitude: aprs_log.longitude,
            latitude: aprs_log.latitude,
        }
    }
}

impl From<AprsLog> for AprsLogRow {
    fn from(aprs_log: AprsLog) -> Self {
        let (state, time, distance, message) = match aprs_log.state {
            AprsState::Travelling { time } => (0, time, 0.0, None),
            AprsState::Approaching { time, distance } => (1, time, distance, None),
            AprsState::Climbing { time, distance } => (2, time, distance, None),
            AprsState::NearSummit {
                time,
                distance,
                message,
            } => (3, time, distance, Some(message)),
            AprsState::OnSummit {
                time,
                distance,
                message,
            } => (4, time, distance, Some(message)),
            AprsState::Descending { time, distance } => (5, time, distance, None),
        };
        AprsLogRow {
            time,
            callsign: aprs_log.callsign.callsign,
            ssid: aprs_log.callsign.ssid.unwrap_or(0) as i64,
            destination: aprs_log.destination,
            distance: Some(distance),
            state,
            message,
            longitude: aprs_log.longitude,
            latitude: aprs_log.latitude,
        }
    }
}
