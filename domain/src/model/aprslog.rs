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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn make_test_time() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2025-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn test_aprs_state_travelling_distance() {
        let state = AprsState::Travelling {
            time: make_test_time(),
        };
        assert_eq!(state.distance(), 0.0);
    }

    #[test]
    fn test_aprs_state_approaching_distance() {
        let state = AprsState::Approaching {
            time: make_test_time(),
            distance: 1500.0,
        };
        assert_eq!(state.distance(), 1500.0);
    }

    #[test]
    fn test_aprs_state_climbing_distance() {
        let state = AprsState::Climbing {
            time: make_test_time(),
            distance: 800.0,
        };
        assert_eq!(state.distance(), 800.0);
    }

    #[test]
    fn test_aprs_state_near_summit_distance() {
        let state = AprsState::NearSummit {
            time: make_test_time(),
            distance: 150.0,
            message: "Approaching summit".to_string(),
        };
        assert_eq!(state.distance(), 150.0);
    }

    #[test]
    fn test_aprs_state_on_summit_distance() {
        let state = AprsState::OnSummit {
            time: make_test_time(),
            distance: 50.0,
            message: "Welcome to summit".to_string(),
        };
        assert_eq!(state.distance(), 50.0);
    }

    #[test]
    fn test_aprs_state_descending_distance() {
        let state = AprsState::Descending {
            time: make_test_time(),
            distance: 300.0,
        };
        assert_eq!(state.distance(), 300.0);
    }

    #[test]
    fn test_aprs_state_time() {
        let time = make_test_time();
        let state = AprsState::Approaching {
            time,
            distance: 1000.0,
        };
        assert_eq!(state.time(), time);
    }

    #[test]
    fn test_aprs_state_message_none_for_travelling() {
        let state = AprsState::Travelling {
            time: make_test_time(),
        };
        assert!(state.message().is_none());
    }

    #[test]
    fn test_aprs_state_message_none_for_approaching() {
        let state = AprsState::Approaching {
            time: make_test_time(),
            distance: 1000.0,
        };
        assert!(state.message().is_none());
    }

    #[test]
    fn test_aprs_state_message_none_for_climbing() {
        let state = AprsState::Climbing {
            time: make_test_time(),
            distance: 500.0,
        };
        assert!(state.message().is_none());
    }

    #[test]
    fn test_aprs_state_message_some_for_near_summit() {
        let msg = "Approaching JA/TK-001".to_string();
        let state = AprsState::NearSummit {
            time: make_test_time(),
            distance: 150.0,
            message: msg.clone(),
        };
        assert_eq!(state.message(), Some(&msg));
    }

    #[test]
    fn test_aprs_state_message_some_for_on_summit() {
        let msg = "Welcome to JA/TK-001".to_string();
        let state = AprsState::OnSummit {
            time: make_test_time(),
            distance: 50.0,
            message: msg.clone(),
        };
        assert_eq!(state.message(), Some(&msg));
    }

    #[test]
    fn test_aprs_state_message_none_for_descending() {
        let state = AprsState::Descending {
            time: make_test_time(),
            distance: 300.0,
        };
        assert!(state.message().is_none());
    }
}
