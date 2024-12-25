use chrono::{DateTime, Utc};
use common::id::UserId;
use domain::model::common::event::{CreateRef, UpdateRef};
use domain::model::pota::POTAReference;
use serde::{Deserialize, Serialize};

#[derive(Debug, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateRefRequest {
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

impl From<CreateRefRequest> for CreateRefRef<POTAReference> {
    fn from(value: CreateRefRequest) -> Self {
        let CreateARefRequest {
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
            requests: vec![POTAReference {
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
            }],
        }
    }
}

#[derive(Debug)]
pub struct WWFFReference {
    pub wwff_code: String,
    pub pota_code: String,
}

#[derive(Debug)]
pub struct POTAActivatorLog {
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

#[derive(Debug)]
pub struct POTAHunterLog {
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
