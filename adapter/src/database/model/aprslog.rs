use chrono::NaiveDateTime;
use domain::model::aprslog::{AprsLog, AprsState};

#[derive(Debug, sqlx::FromRow)]
pub struct AprsLogImpl {
    pub time: NaiveDateTime,
    pub callsign: String,
    pub ssid: i64,
    pub destination: String,
    pub distance: f64,
    pub state: i64,
    pub message: Option<String>,
    pub longitude: f64,
    pub latitude: f64,
}

impl From<AprsLogImpl> for AprsLog {
    fn from(aprs_log: AprsLogImpl) -> Self {
        let state = match aprs_log.state {
            0 => AprsState::Approaching {
                time: aprs_log.time,
                distance: aprs_log.distance,
            },
            1 => AprsState::Climbing {
                time: aprs_log.time,
                distance: aprs_log.distance,
            },
            2 => AprsState::NearSummit {
                time: aprs_log.time,
                distance: aprs_log.distance,
                message: aprs_log.message,
            },
            3 => AprsState::OnSummit {
                time: aprs_log.time,
                distance: aprs_log.distance,
                message: aprs_log.message,
            },
            4 => AprsState::Descending {
                time: aprs_log.time,
                distance: aprs_log.distance,
            },
            _ => panic!("Invalid state"),
        };
        AprsLog {
            callsign: aprs_log.callsign,
            ssid: aprs_log.ssid as u32,
            destination: aprs_log.destination,
            state,
            longitude: aprs_log.longitude,
            latitude: aprs_log.latitude,
        }
    }
}

impl From<AprsLog> for AprsLogImpl {
    fn from(aprs_log: AprsLog) -> Self {
        let (state, time, distance, message) = match aprs_log.state {
            AprsState::Approaching { time, distance } => (0, time, distance, None),
            AprsState::Climbing { time, distance } => (1, time, distance, None),
            AprsState::NearSummit {
                time,
                distance,
                message,
            } => (2, time, distance, message),
            AprsState::OnSummit {
                time,
                distance,
                message,
            } => (3, time, distance, message),
            AprsState::Descending { time, distance } => (4, time, distance, None),
        };
        AprsLogImpl {
            time,
            callsign: aprs_log.callsign,
            ssid: aprs_log.ssid as i64,
            destination: aprs_log.destination,
            distance,
            state,
            message,
            longitude: aprs_log.longitude,
            latitude: aprs_log.latitude,
        }
    }
}
