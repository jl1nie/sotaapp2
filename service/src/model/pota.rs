use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use common::utils::{maidenhead, parse_date_flexible};
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
        let lon = longitude.unwrap_or_default();
        let lat = latitude.unwrap_or_default();
        Self {
            pota_code: pota_code.unwrap_or_default(),
            wwff_code: wwff_code.unwrap_or_default(),
            park_name,
            park_name_j,
            park_location,
            park_locid: park_locid.unwrap_or_default(),
            park_type,
            park_inactive: park_inactive.is_some(),
            park_area,
            longitude: lon,
            latitude: lat,
            maidenhead: maidenhead(lon, lat),
            update,
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct POTAAllCSVFile {
    pub reference: String,
    pub name: String,
    pub active: String,
    pub entity_id: String,
    pub location_desc: String,
    pub latitude: String,
    pub longitude: String,
    pub grid: String,
}

impl TryFrom<POTAAllCSVFile> for PotaReference {
    type Error = String;
    fn try_from(value: POTAAllCSVFile) -> Result<Self, Self::Error> {
        let POTAAllCSVFile {
            reference,
            name,
            active,
            location_desc,
            latitude,
            longitude,
            grid,
            ..
        } = value;

        let update: DateTime<Utc> = Utc::now();
        let park_inactive = !&active.contains("1");

        Ok(Self {
            pota_code: reference.clone(),
            wwff_code: "".to_string(),
            park_name: name.clone(),
            park_name_j: name,
            park_location: "".to_string(),
            park_locid: location_desc,
            park_type: "".to_string(),
            park_inactive,
            park_area: 0,
            longitude: longitude
                .parse::<f64>()
                .map_err(|e| format!("parse error ={} {}", reference, e))?,
            latitude: latitude
                .parse::<f64>()
                .map_err(|e| format!("parse error ={} {}", reference, e))?,
            maidenhead: grid,
            update,
        })
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
    pub fn to_log(log_id: LogId, value: POTAActivatorLogCSV) -> Option<PotaActLog> {
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
        let date = parse_date_flexible(&first_qso_date)?;
        Some(PotaActLog {
            log_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date: date,
            attempts,
            activations,
            qsos,
        })
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
    pub fn to_log(log_id: LogId, value: POTAHunterLogCSV) -> Option<PotaHuntLog> {
        let POTAHunterLogCSV {
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date,
            qsos,
        } = value;
        let date = parse_date_flexible(&first_qso_date)?;
        Some(PotaHuntLog {
            log_id,
            dx_entity,
            location,
            hasc,
            pota_code,
            park_name,
            first_qso_date: date,
            qsos,
        })
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
