use chrono::{DateTime, NaiveDate, Utc};

use super::{id::UserId, Maidenhead};

pub struct SummitCode(String);
impl SummitCode {
    pub fn new(code: String) -> Self {
        Self(code)
    }
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct SotaReference {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SotaLog {
    pub user_id: UserId,
    pub my_callsign: String,
    pub operator: String,
    pub my_summit_code: Option<String>,
    pub time: DateTime<Utc>,
    pub frequency: String,
    pub mode: String,
    pub his_callsign: String,
    pub his_summit_code: Option<String>,
    pub comment: Option<String>,
    pub update: DateTime<Utc>,
}
