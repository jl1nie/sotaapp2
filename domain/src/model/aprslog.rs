use aprs_message::AprsCallsign;
use chrono::{DateTime, NaiveDateTime, Utc};

#[derive(Debug, Clone)]
pub enum AprsState {
    Travelling {
        time: NaiveDateTime,
    },
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
        message: String,
    },
    OnSummit {
        time: NaiveDateTime,
        distance: f64,
        message: String,
    },
    Descending {
        time: NaiveDateTime,
        distance: f64,
    },
}

impl AprsState {
    pub fn distance(&self) -> f64 {
        match self {
            Self::Travelling { .. } => 0.0,
            Self::Approaching { distance, .. } => *distance,
            Self::Climbing { distance, .. } => *distance,
            Self::NearSummit { distance, .. } => *distance,
            Self::OnSummit { distance, .. } => *distance,
            Self::Descending { distance, .. } => *distance,
        }
    }

    pub fn time(&self) -> NaiveDateTime {
        match self {
            Self::Travelling { time } => *time,
            Self::Approaching { time, .. } => *time,
            Self::Climbing { time, .. } => *time,
            Self::NearSummit { time, .. } => *time,
            Self::OnSummit { time, .. } => *time,
            Self::Descending { time, .. } => *time,
        }
    }

    pub fn message(&self) -> Option<&String> {
        match self {
            Self::Travelling { .. } => None,
            Self::Approaching { .. } => None,
            Self::Climbing { .. } => None,
            Self::NearSummit { message, .. } => Some(message),
            Self::OnSummit { message, .. } => Some(message),
            Self::Descending { .. } => None,
        }
    }
}

#[derive(Debug)]
pub struct AprsLog {
    pub callsign: AprsCallsign,
    pub destination: Option<String>,
    pub state: AprsState,
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug)]
pub struct AprsTrack {
    pub coordinates: Vec<(f64, f64)>,
    pub callsign: AprsCallsign,
    pub lastseen: DateTime<Utc>,
    pub distance: Option<f64>,
    pub summit: Option<String>,
    pub spot_summit: Option<String>,
    pub spot_time: Option<DateTime<Utc>>,
    pub spot_freq: Option<String>,
    pub spot_mode: Option<String>,
    pub spot_comment: Option<String>,
}
