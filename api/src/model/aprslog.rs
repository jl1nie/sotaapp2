use serde::Serialize;
use typeshare::typeshare;

use domain::model::aprslog::{AprsLog, AprsState, AprsTrack};

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct AprsLogView {
    pub time: String,
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
            time: time.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use aprs_message::AprsCallsign;
    use chrono::{NaiveDateTime, TimeZone, Utc};

    fn make_test_time() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn create_test_aprs_callsign(callsign: &str, ssid: Option<u32>) -> AprsCallsign {
        AprsCallsign {
            callsign: callsign.to_string(),
            ssid,
        }
    }

    // =====================================================
    // AprsLogView 変換テスト
    // =====================================================

    #[test]
    fn test_aprs_log_view_from_travelling() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: Some("JA/TK-001".to_string()),
            state: AprsState::Travelling {
                time: make_test_time(),
            },
            longitude: 139.2438,
            latitude: 35.6251,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.callsign, "JA1ABC");
        assert_eq!(view.ssid, 7);
        assert_eq!(view.destination, "JA/TK-001");
        assert_eq!(view.state, "Travelling");
        assert_eq!(view.distance, 0.0);
        assert!((view.longitude - 139.2438).abs() < 0.0001);
        assert!((view.latitude - 35.6251).abs() < 0.0001);
    }

    #[test]
    fn test_aprs_log_view_from_approaching() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", None),
            destination: None,
            state: AprsState::Approaching {
                time: make_test_time(),
                distance: 1500.0,
            },
            longitude: 139.0,
            latitude: 35.0,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.ssid, 0); // None -> default 0
        assert_eq!(view.destination, ""); // None -> empty string
        assert_eq!(view.state, "Approaching");
        assert_eq!(view.distance, 1500.0);
    }

    #[test]
    fn test_aprs_log_view_from_climbing() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1XYZ", Some(9)),
            destination: Some("JA/TK-002".to_string()),
            state: AprsState::Climbing {
                time: make_test_time(),
                distance: 800.0,
            },
            longitude: 139.5,
            latitude: 35.5,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.state, "Climbing");
        assert_eq!(view.distance, 800.0);
    }

    #[test]
    fn test_aprs_log_view_from_near_summit() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: Some("JA/TK-001".to_string()),
            state: AprsState::NearSummit {
                time: make_test_time(),
                distance: 150.0,
                message: "Almost there".to_string(),
            },
            longitude: 139.2438,
            latitude: 35.6251,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.state, "NearSummit");
        assert_eq!(view.distance, 150.0);
    }

    #[test]
    fn test_aprs_log_view_from_on_summit() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: Some("JA/TK-001".to_string()),
            state: AprsState::OnSummit {
                time: make_test_time(),
                distance: 50.0,
                message: "On summit!".to_string(),
            },
            longitude: 139.2438,
            latitude: 35.6251,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.state, "OnSummit");
        assert_eq!(view.distance, 50.0);
    }

    #[test]
    fn test_aprs_log_view_from_descending() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: Some("JA/TK-001".to_string()),
            state: AprsState::Descending {
                time: make_test_time(),
                distance: 300.0,
            },
            longitude: 139.2438,
            latitude: 35.6251,
        };

        let view: AprsLogView = log.into();

        assert_eq!(view.state, "Descending");
        assert_eq!(view.distance, 300.0);
    }

    #[test]
    fn test_aprs_log_view_time_format() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: None,
            state: AprsState::Travelling {
                time: make_test_time(),
            },
            longitude: 139.0,
            latitude: 35.0,
        };

        let view: AprsLogView = log.into();

        assert!(view.time.contains("2024-06-15"));
        assert!(view.time.contains("10:30:00"));
    }

    // =====================================================
    // Track 変換テスト
    // =====================================================

    #[test]
    fn test_track_from_aprs_track() {
        let track = AprsTrack {
            coordinates: vec![(139.0, 35.0), (139.1, 35.1), (139.2, 35.2)],
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            distance: Some(500.0),
            summit: Some("JA/TK-001".to_string()),
            spot_summit: Some("JA/TK-001".to_string()),
            spot_time: Some(Utc.with_ymd_and_hms(2024, 6, 15, 10, 00, 0).unwrap()),
            spot_freq: Some("14.285".to_string()),
            spot_mode: Some("SSB".to_string()),
            spot_comment: Some("CQ CQ".to_string()),
        };

        let view: Track = track.into();

        assert_eq!(view.type_, "Feature");
        assert_eq!(view.geometry.type_, "LineString");
        assert_eq!(view.geometry.coordinates.len(), 3);
        assert_eq!(view.geometry.coordinates[0], vec![139.0, 35.0]);
        assert_eq!(view.properties.callsign, "JA1ABC");
        assert_eq!(view.properties.ssid, Some("7".to_string()));
        assert_eq!(view.properties.distance, 500);
        assert_eq!(view.properties.summit, "JA/TK-001");
        assert_eq!(view.properties.spot_summit, Some("JA/TK-001".to_string()));
        assert_eq!(view.properties.spot_freq, Some("14.285".to_string()));
        assert_eq!(view.properties.spot_mode, Some("SSB".to_string()));
    }

    #[test]
    fn test_track_from_aprs_track_no_ssid() {
        let track = AprsTrack {
            coordinates: vec![(139.0, 35.0)],
            callsign: create_test_aprs_callsign("JA1ABC", None),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            distance: None,
            summit: None,
            spot_summit: None,
            spot_time: None,
            spot_freq: None,
            spot_mode: None,
            spot_comment: None,
        };

        let view: Track = track.into();

        assert!(view.properties.ssid.is_none());
        assert_eq!(view.properties.distance, 0);
        assert_eq!(view.properties.summit, "");
        assert!(view.properties.spot_summit.is_none());
        assert!(view.properties.spot_time.is_none());
    }

    #[test]
    fn test_track_lastseen_format() {
        let track = AprsTrack {
            coordinates: vec![],
            callsign: create_test_aprs_callsign("JA1ABC", None),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            distance: None,
            summit: None,
            spot_summit: None,
            spot_time: None,
            spot_freq: None,
            spot_mode: None,
            spot_comment: None,
        };

        let view: Track = track.into();

        // RFC3339形式
        assert!(view.properties.lastseen.contains("2024-06-15"));
        assert!(view.properties.lastseen.contains("10:30:00"));
    }

    // =====================================================
    // Tracks 構造体テスト
    // =====================================================

    #[test]
    fn test_tracks_structure() {
        let track1 = AprsTrack {
            coordinates: vec![(139.0, 35.0)],
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            distance: Some(100.0),
            summit: Some("JA/TK-001".to_string()),
            spot_summit: None,
            spot_time: None,
            spot_freq: None,
            spot_mode: None,
            spot_comment: None,
        };

        let track2 = AprsTrack {
            coordinates: vec![(140.0, 36.0)],
            callsign: create_test_aprs_callsign("JA2XYZ", Some(9)),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 11, 00, 0).unwrap(),
            distance: Some(200.0),
            summit: Some("JA/TK-002".to_string()),
            spot_summit: None,
            spot_time: None,
            spot_freq: None,
            spot_mode: None,
            spot_comment: None,
        };

        let tracks = Tracks {
            tracks: vec![track1.into(), track2.into()],
        };

        assert_eq!(tracks.tracks.len(), 2);
        assert_eq!(tracks.tracks[0].properties.callsign, "JA1ABC");
        assert_eq!(tracks.tracks[1].properties.callsign, "JA2XYZ");
    }

    // =====================================================
    // JSON シリアライズテスト
    // =====================================================

    #[test]
    fn test_aprs_log_view_json_serialization() {
        let log = AprsLog {
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            destination: Some("JA/TK-001".to_string()),
            state: AprsState::OnSummit {
                time: make_test_time(),
                distance: 50.0,
                message: "Summit".to_string(),
            },
            longitude: 139.2438,
            latitude: 35.6251,
        };

        let view: AprsLogView = log.into();
        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力される
        assert!(json.contains("\"callsign\":\"JA1ABC\""));
        assert!(json.contains("\"ssid\":7"));
        assert!(json.contains("\"state\":\"OnSummit\""));
    }

    #[test]
    fn test_track_json_serialization() {
        let track = AprsTrack {
            coordinates: vec![(139.0, 35.0)],
            callsign: create_test_aprs_callsign("JA1ABC", Some(7)),
            lastseen: Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap(),
            distance: Some(100.0),
            summit: Some("JA/TK-001".to_string()),
            spot_summit: None,
            spot_time: None,
            spot_freq: None,
            spot_mode: None,
            spot_comment: None,
        };

        let view: Track = track.into();
        let json = serde_json::to_string(&view).unwrap();

        // GeoJSON形式
        assert!(json.contains("\"type\":\"Feature\""));
        assert!(json.contains("\"type\":\"LineString\""));
        assert!(json.contains("\"coordinates\":[[139.0,35.0]]"));
    }
}
