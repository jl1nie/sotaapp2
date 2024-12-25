use chrono::{DateTime, Utc};
use domain::model::pota::{POTAReference, WWFFReference};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTACSVFile {
    pub pota_code: String,
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
pub struct WWFFCSVFile {
    pub wwff_code: String,
    pub pota_code: String,
}
impl From<WWFFCSVFile> for WWFFReference {
    fn from(value: WWFFCSVFile) -> Self {
        let WWFFCSVFile {
            wwff_code,
            pota_code,
        } = value;
        let update: DateTime<Utc> = Utc::now();
        Self {
            wwff_code,
            pota_code,
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
