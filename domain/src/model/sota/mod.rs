use serde::{Deserialize, Serialize};

pub mod event;
use crate::model::common::activation::{Alert, Spot};
pub type SOTAAlert = Alert;
pub type SOTASpot = Spot;

pub struct SummitCode(String);
impl SummitCode {
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SOTAReference {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SOTARefOptInfo {
    pub summit_code: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
}
