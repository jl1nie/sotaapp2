use chrono::NaiveDateTime;

#[derive(Debug)]
pub enum AprsState {
    Approaching { time: NaiveDateTime, distance: f64 },
    Climbing { time: NaiveDateTime, distance: f64 },
    NearSummit { time: NaiveDateTime, distance: f64 },
    OnSummit { time: NaiveDateTime, distance: f64 },
    Descending { time: NaiveDateTime, distance: f64 },
}

#[derive(Debug)]
pub struct AprsLog {
    pub callsign: String,
    pub ssid: u32,
    pub destination: String,
    pub state: AprsState,
    pub longitude: f64,
    pub latitude: f64,
}
