use chrono::{DateTime, Utc};

use crate::model::pota::PotaRefLog;
use crate::model::AwardProgram;

#[derive(Debug)]
pub struct Alert {
    pub program: AwardProgram,
    pub alert_id: i32,
    pub user_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub location: String,
    pub activator: String,
    pub activator_name: Option<String>,
    pub operator: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub frequencies: String,
    pub comment: Option<String>,
    pub poster: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Spot {
    pub program: AwardProgram,
    pub spot_id: i32,
    pub reference: String,
    pub reference_detail: String,
    pub activator: String,
    pub activator_name: Option<String>,
    pub operator: String,
    pub spot_time: DateTime<Utc>,
    pub frequency: String,
    pub mode: String,
    pub spotter: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpotLog {
    pub spot: Spot,
    pub qsos: Option<i32>,
}

impl SpotLog {
    pub fn new(s: Spot, r: Option<PotaRefLog>) -> Self {
        if let Some(r) = r {
            SpotLog {
                spot: s,
                qsos: r.qsos,
            }
        } else {
            SpotLog {
                spot: s,
                qsos: None,
            }
        }
    }
}
