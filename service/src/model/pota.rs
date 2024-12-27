use chrono::{DateTime, NaiveDate, Utc};
use domain::model::common::id::UserId;
use domain::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTACSVFile {
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
}

impl From<POTACSVFile> for POTAReference {
    fn from(value: POTACSVFile) -> Self {
        let POTACSVFile {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_status,
            park_area,
            longitude,
            lattitude,
        } = value;
        let update: DateTime<Utc> = Utc::now();
        Self {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_status,
            park_area,
            longitude,
            lattitude,
            update,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTAActivatorLogCSV {
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: String,
    pub attempts: i32,
    pub activations: i32,
    pub qsos: i32,
}

impl POTAActivatorLogCSV {
    pub fn to_log(user_id: UserId, value: POTAActivatorLogCSV) -> POTAActivatorLog {
        let POTAActivatorLogCSV {
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            attempts,
            activations,
            qsos,
        } = value;
        let upload: DateTime<Utc> = Utc::now();
        let first_qso_date = NaiveDate::parse_from_str(&first_qso_date, "%Y-%m-%d").unwrap();
        POTAActivatorLog {
            user_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            attempts,
            activations,
            qsos,
            upload,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTAHunterLogCSV {
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: String,
    pub qsos: i32,
}
impl POTAHunterLogCSV {
    pub fn to_log(user_id: UserId, value: POTAHunterLogCSV) -> POTAHunterLog {
        let POTAHunterLogCSV {
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            qsos,
        } = value;
        let upload: DateTime<Utc> = Utc::now();
        let first_qso_date = NaiveDate::parse_from_str(&first_qso_date, "%Y-%m-%d").unwrap();
        POTAHunterLog {
            user_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            qsos,
            upload,
        }
    }
}
pub struct UploadPOTACSV {
    pub data: String,
}
pub struct UploadWWFFCSV {
    pub data: String,
}

pub struct UploadActivatorCSV {
    pub data: String,
}

pub struct UploadHunterCSV {
    pub data: String,
}
