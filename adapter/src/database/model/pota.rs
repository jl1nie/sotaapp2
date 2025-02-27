use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use domain::model::id::{LogId, UserId};
use domain::model::pota::{
    POTAActivatorLog, POTAHunterLog, POTALogUser, POTAReference, POTAReferenceWithLog,
};
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
    pub park_inactive: bool,
    pub park_area: i64,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub maidenhead: String,
    pub update: DateTime<Utc>,
}

impl From<POTAReference> for POTAReferenceImpl {
    fn from(r: POTAReference) -> Self {
        POTAReferenceImpl {
            pota_code: r.pota_code,
            wwff_code: r.wwff_code,
            park_name: r.park_name,
            park_name_j: r.park_name_j,
            park_location: r.park_location,
            park_locid: r.park_locid,
            park_type: r.park_type,
            park_inactive: r.park_inactive,
            park_area: r.park_area as i64,
            longitude: r.longitude,
            latitude: r.latitude,
            maidenhead: r.maidenhead,
            update: r.update,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct POTAReferenceWithLogImpl {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i64,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub maidenhead: String,
    pub attempts: Option<i32>,
    pub activations: Option<i32>,
    pub first_qso_date: Option<NaiveDate>,
    pub qsos: Option<i32>,
}

impl From<POTAReferenceWithLogImpl> for POTAReferenceWithLog {
    fn from(r: POTAReferenceWithLogImpl) -> Self {
        POTAReferenceWithLog {
            pota_code: r.pota_code,
            wwff_code: r.wwff_code,
            park_name: r.park_name,
            park_name_j: r.park_name_j,
            park_location: r.park_location,
            park_locid: r.park_locid,
            park_type: r.park_type,
            park_inactive: r.park_inactive,
            park_area: r.park_area as i32,
            longitude: r.longitude,
            latitude: r.latitude,
            maidenhead: r.maidenhead,
            attempts: r.attempts,
            activations: r.activations,
            first_qso_date: r.first_qso_date,
            qsos: r.qsos,
        }
    }
}

impl From<POTAReferenceImpl> for POTAReference {
    fn from(r: POTAReferenceImpl) -> Self {
        POTAReference {
            pota_code: r.pota_code,
            wwff_code: r.wwff_code,
            park_name: r.park_name,
            park_name_j: r.park_name_j,
            park_location: r.park_location,
            park_locid: r.park_locid,
            park_type: r.park_type,
            park_inactive: r.park_inactive,
            park_area: r.park_area as i32,
            longitude: r.longitude,
            latitude: r.latitude,
            maidenhead: r.maidenhead,
            update: r.update,
        }
    }
}

#[derive(Debug, sqlx::Type)]
#[repr(i32)]
pub enum POTALogType {
    ActivatorLog = 1,
    HunterLog = 2,
}
impl TryFrom<i32> for POTALogType {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(POTALogType::ActivatorLog),
            2 => Ok(POTALogType::HunterLog),
            _ => Err(()),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct POTALogImpl {
    pub log_id: LogId,
    pub log_type: POTALogType,
    pub dx_entity: String,
    pub location: String,
    pub hasc: String,
    pub pota_code: String,
    pub park_name: String,
    pub first_qso_date: NaiveDate,
    pub attempts: Option<i32>,
    pub activations: Option<i32>,
    pub qsos: i32,
}

impl From<POTAActivatorLog> for POTALogImpl {
    fn from(l: POTAActivatorLog) -> Self {
        POTALogImpl {
            log_id: l.log_id,
            log_type: POTALogType::ActivatorLog,
            dx_entity: l.dx_entity,
            location: l.location,
            hasc: l.hasc,
            pota_code: l.pota_code,
            park_name: l.park_name,
            first_qso_date: l.first_qso_date,
            attempts: Some(l.attempts),
            activations: Some(l.activations),
            qsos: l.qsos,
        }
    }
}

impl From<POTAHunterLog> for POTALogImpl {
    fn from(l: POTAHunterLog) -> Self {
        POTALogImpl {
            log_id: l.log_id,
            log_type: POTALogType::HunterLog,
            dx_entity: l.dx_entity,
            location: l.location,
            hasc: l.hasc,
            pota_code: l.pota_code,
            park_name: l.park_name,
            first_qso_date: l.first_qso_date,
            attempts: None,
            activations: None,
            qsos: l.qsos,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct POTALogUserImpl {
    pub user_id: UserId,
    pub log_id: LogId,
    pub update: NaiveDateTime,
}

impl From<POTALogUser> for POTALogUserImpl {
    fn from(l: POTALogUser) -> Self {
        POTALogUserImpl {
            user_id: l.user_id,
            log_id: l.log_id,
            update: l.update,
        }
    }
}
