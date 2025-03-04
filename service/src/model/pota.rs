use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use common::utils::maidenhead;
use domain::model::id::LogId;
use domain::model::pota::{PotaActLog, PotaHuntLog, PotaReference};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTACSVFile {
    pub pota_code: Option<String>,
    pub wwff_code: Option<String>,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: Option<String>,
    pub park_type: String,
    pub park_inactive: Option<String>,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

impl From<POTACSVFile> for PotaReference {
    fn from(value: POTACSVFile) -> Self {
        let POTACSVFile {
            pota_code,
            wwff_code,
            park_name,
            park_name_j,
            park_location,
            park_locid,
            park_type,
            park_inactive,
            park_area,
            longitude,
            latitude,
        } = value;
        let update: DateTime<Utc> = Utc::now();
        Self {
            pota_code: pota_code.unwrap_or("".to_string()),
            wwff_code: wwff_code.unwrap_or("".to_string()),
            park_name,
            park_name_j,
            park_location,
            park_locid: park_locid.unwrap_or("".to_string()),
            park_type,
            park_inactive: park_inactive.is_some(),
            park_area,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude.unwrap_or_default(), latitude.unwrap_or_default()),
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
    pub fn to_log(log_id: LogId, value: POTAActivatorLogCSV) -> PotaActLog {
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
        let first_qso_date = NaiveDate::parse_from_str(&first_qso_date, "%Y-%m-%d").unwrap();
        PotaActLog {
            log_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            attempts,
            activations,
            qsos,
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
    pub fn to_log(log_id: LogId, value: POTAHunterLogCSV) -> PotaHuntLog {
        let POTAHunterLogCSV {
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            qsos,
        } = value;
        let first_qso_date = NaiveDate::parse_from_str(&first_qso_date, "%Y-%m-%d").unwrap();
        PotaHuntLog {
            log_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            qsos,
        }
    }
}
pub struct UploadPOTAReference {
    pub data: String,
}
pub struct UploadWWFFReference {
    pub data: String,
}
#[derive(Debug)]
pub struct UploadPOTALog {
    pub activator_logid: String,
    pub hunter_logid: String,
    pub data: String,
}
