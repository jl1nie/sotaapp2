use aprs_message::AprsCallsign;
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub enum AprsState {
    Approaching {
        time: NaiveDateTime,
        distance: f64,
    },
    Climbing {
        time: NaiveDateTime,
        distance: f64,
    },
    NearSummit {
        time: NaiveDateTime,
        distance: f64,
        message: Option<String>,
    },
    OnSummit {
        time: NaiveDateTime,
        distance: f64,
        message: Option<String>,
    },
    Descending {
        time: NaiveDateTime,
        distance: f64,
    },
}

#[derive(Debug)]
pub struct AprsLog {
    pub callsign: AprsCallsign,
    pub destination: String,
    pub state: AprsState,
    pub longitude: f64,
    pub latitude: f64,
}
