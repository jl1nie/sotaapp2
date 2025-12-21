use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use common::utils::{call_to_operator, maidenhead, parse_date_flexible};
use domain::model::{
    id::UserId,
    sota::{SotaLog, SotaReference},
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
    pub longitude: f64,
    pub latitude: f64,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SOTASummitCSV> for SotaReference {
    fn from(csv: SOTASummitCSV) -> SotaReference {
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
        // デフォルト日付（1970-01-01）をフォールバックとして使用
        let default_date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid default date");
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
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from: parse_date_flexible(&valid_from).unwrap_or(default_date),
            valid_to: parse_date_flexible(&valid_to).unwrap_or(default_date),
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
    pub latitude: f64,
    pub longitude: f64,
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
    pub fn to_log(user_id: UserId, value: SOTALogCSV) -> SotaLog {
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

        // デフォルト日時（1900-01-01 00:00）
        let default_datetime = NaiveDateTime::parse_from_str("01/01/1900 0000", "%d/%m/%Y %H%M")
            .expect("valid default datetime");
        let date_str = format!("{} {}", date, time);

        let parsed = ["%d/%m/%Y %H:%M", "%d/%m/%Y %H%M"]
            .iter()
            .find_map(|pat| NaiveDateTime::parse_from_str(&date_str, pat).ok())
            .unwrap_or(default_datetime);

        let time = Utc.from_utc_datetime(&parsed);

        let operator = call_to_operator(&my_callsign);
        let update = Utc::now();
        SotaLog {
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
