use chrono::{DateTime, NaiveDate, Utc};
use domain::model::id::UserId;
use domain::model::sota::{SOTALog, SOTAReference};
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct SOTAReferenceImpl {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub maidenhead: String,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SOTAReference> for SOTAReferenceImpl {
    fn from(s: SOTAReference) -> Self {
        let SOTAReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft: i32,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = s;
        Self {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft: i32,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}
impl From<SOTAReferenceImpl> for SOTAReference {
    fn from(s: SOTAReferenceImpl) -> Self {
        let SOTAReferenceImpl {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft: i32,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = s;
        Self {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft: i32,
            grid_ref1,
            grid_ref2,
            longitude,
            latitude,
            maidenhead,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct SOTALogImpl {
    pub user_id: Uuid,
    pub my_callsign: String,
    pub operator: String,
    pub my_summit_code: Option<String>,
    pub time: DateTime<Utc>,
    pub frequency: String,
    pub mode: String,
    pub his_callsign: String,
    pub his_summit_code: Option<String>,
    pub comment: Option<String>,
    pub update: DateTime<Utc>,
}

impl From<SOTALog> for SOTALogImpl {
    fn from(value: SOTALog) -> Self {
        let SOTALog {
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
        } = value;
        Self {
            user_id: user_id.raw(),
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

impl From<SOTALogImpl> for SOTALog {
    fn from(value: SOTALogImpl) -> Self {
        let SOTALogImpl {
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
        } = value;
        Self {
            user_id: UserId::from(user_id),
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
