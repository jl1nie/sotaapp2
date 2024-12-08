use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::common::{Alert, Spot};

pub type POTAAlert = Alert;
pub type POTASpot = Spot;

pub mod event;
pub struct ParkCode(String);
impl ParkCode {
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct POTAReference {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct POTAActivatorLog {
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub attempts: i32,
    pub activations: i32,
    pub qsos: i32,
}

#[derive(Debug, Deserialize)]
pub struct POTAHunterLog {
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub qsos: i32,
}
