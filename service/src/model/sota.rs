use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use common::utils::{call_to_operator, maidenhead};
use domain::model::{
    id::UserId,
    sota::{SOTALog, SOTAReference},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SOTASummitCSV {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SOTASummitCSV> for SOTAReference {
    fn from(csv: SOTASummitCSV) -> SOTAReference {
        let SOTASummitCSV {
            summit_code,
            association_name,
            region_name,
            summit_name,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = csv;
        Self {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j: None,
            city: None,
            city_j: None,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead: maidenhead(longitude.unwrap_or_default(), latitude.unwrap_or_default()),
            points,
            bonus_points,
            valid_from: NaiveDate::parse_from_str(&valid_from, "%d/%m/%Y").unwrap(),
            valid_to: NaiveDate::parse_from_str(&valid_to, "%d/%m/%Y").unwrap(),
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SOTASumitOptCSV {
    pub summit_code: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub points: i32,
    pub alt_m: i32,
    pub summit_name: String,
    pub city: String,
    pub summit_name_j: String,
    pub city_j: String,
}

#[derive(Debug, Deserialize)]
pub struct SOTALogCSV {
    pub version: String,
    pub my_callsign: String,
    pub my_summit_code: Option<String>,
    pub date: String,
    pub time: String,
    pub frequency: String,
    pub mode: String,
    pub his_callsign: String,
    pub his_summit_code: Option<String>,
    pub comment: Option<String>,
}

impl SOTALogCSV {
    pub fn to_log(user_id: UserId, value: SOTALogCSV) -> SOTALog {
        let SOTALogCSV {
            version,
            my_callsign,
            my_summit_code,
            date,
            time,
            frequency,
            mode,
            his_callsign,
            his_summit_code,
            comment,
        } = value;

        if version != "V2" {
            tracing::warn!("SOTA LOG CSV format version error: {}", version)
        }

        let mut parsed = NaiveDateTime::parse_from_str("01/01/1900 0000", "%d/%m/%Y %H%M").unwrap();
        let date = date.clone() + " " + &time;

        for pat in ["%d/%m/%Y %H:%M", "%d/%m/%Y %H%M"] {
            if let Ok(update) = NaiveDateTime::parse_from_str(&date, pat) {
                parsed = update;
                break;
            }
        }

        let time = Utc.from_utc_datetime(&parsed);

        let operator = call_to_operator(&my_callsign);
        let update = Utc::now();
        SOTALog {
            user_id,
            my_callsign,
            operator,
            my_summit_code,
            time,
            frequency,
            mode,
            his_callsign,
            his_summit_code,
            comment,
            update,
        }
    }
}

pub struct UploadSOTASummit {
    pub data: String,
}

pub struct UploadSOTASummitOpt {
    pub data: String,
}

pub struct UploadSOTALog {
    pub data: String,
}
