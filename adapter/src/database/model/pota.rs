use chrono::NaiveDate;
use domain::model::common::id::UserId;
use sqlx::FromRow;

#[derive(Debug)]
pub struct ParkCode(String);
impl ParkCode {
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, FromRow)]
pub struct POTAReferenceImpl {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_status: bool,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
    pub update: NaiveDate,
}

#[derive(Debug, FromRow)]
pub struct POTAActivatorLogImpl {
    pub user_id: UserId,
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub attempts: i32,
    pub activations: i32,
    pub qsos: i32,
    pub upload: NaiveDate,
}

#[derive(Debug, FromRow)]
pub struct POTAHunterLogImpl {
    pub user_id: UserId,
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub qsos: i32,
    pub upload: NaiveDate,
}
