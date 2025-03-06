use aprs_message::AprsCallsign;
use chrono::{DateTime, NaiveDateTime, Utc};

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

impl AprsState {
    pub fn distance(&self) -> f64 {
        match self {
            Self::Approaching { distance, .. } => *distance,
            Self::Climbing { distance, .. } => *distance,
            Self::NearSummit { distance, .. } => *distance,
            Self::OnSummit { distance, .. } => *distance,
            Self::Descending { distance, .. } => *distance,
        }
    }

    pub fn time(&self) -> NaiveDateTime {
        match self {
            Self::Approaching { time, .. } => *time,
            Self::Climbing { time, .. } => *time,
            Self::NearSummit { time, .. } => *time,
            Self::OnSummit { time, .. } => *time,
            Self::Descending { time, .. } => *time,
        }
    }
}

#[derive(Debug)]
pub struct AprsLog {
    pub callsign: AprsCallsign,
    pub destination: String,
    pub state: AprsState,
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug)]
pub struct AprsTrack {
    pub coordinates: Vec<(f64, f64)>,
    pub callsign: AprsCallsign,
    pub lastseen: DateTime<Utc>,
    pub distance: f64,
    pub summit: String,
    pub spot_summit: Option<String>,
    pub spot_time: Option<DateTime<Utc>>,
    pub spot_freq: Option<String>,
    pub spot_mode: Option<String>,
    pub spot_comment: Option<String>,
}
