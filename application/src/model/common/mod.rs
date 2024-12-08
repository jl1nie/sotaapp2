use chrono::{DateTime, Utc};
pub mod event;

pub struct Alert {
    alert_id: i32,
    user_id: i32,
    reference: String,
    reference_detail: String,
    location: String,
    activator: String,
    activator_name: Option<String>,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    frequencies: String,
    comment: Option<String>,
    poster: Option<String>,
}

pub struct Spot {
    spot_id: i32,
    reference: String,
    reference_detail: String,
    activator: String,
    activator_name: Option<String>,
    spot_time: DateTime<Utc>,
    frequency: String,
    mode: String,
    spotter: String,
    comment: Option<String>,
}
