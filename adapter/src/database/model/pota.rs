use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use domain::model::id::{LogId, UserId};
use domain::model::pota::{PotaActLog, PotaHuntLog, PotaLogHist, PotaRefLog, PotaReference};
use sqlx::FromRow;

#[derive(Debug)]
pub struct ParkCode(String);
impl ParkCode {
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, FromRow)]
pub struct PotaReferenceRow {
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

impl From<PotaReference> for PotaReferenceRow {
    fn from(r: PotaReference) -> Self {
        PotaReferenceRow {
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
pub struct PotaRefLogRow {
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

impl From<PotaRefLogRow> for PotaRefLog {
    fn from(r: PotaRefLogRow) -> Self {
        PotaRefLog {
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

impl From<PotaReferenceRow> for PotaReference {
    fn from(r: PotaReferenceRow) -> Self {
        PotaReference {
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
pub enum PotaLogType {
    ActivatorLog = 1,
    HunterLog = 2,
}
impl TryFrom<i32> for PotaLogType {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PotaLogType::ActivatorLog),
            2 => Ok(PotaLogType::HunterLog),
            _ => Err(()),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PotaLogRow {
    pub log_id: LogId,
    pub log_type: PotaLogType,
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

impl From<PotaActLog> for PotaLogRow {
    fn from(l: PotaActLog) -> Self {
        PotaLogRow {
            log_id: l.log_id,
            log_type: PotaLogType::ActivatorLog,
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

impl From<PotaHuntLog> for PotaLogRow {
    fn from(l: PotaHuntLog) -> Self {
        PotaLogRow {
            log_id: l.log_id,
            log_type: PotaLogType::HunterLog,
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
pub struct PotaLogHistRow {
    pub user_id: UserId,
    pub log_id: LogId,
    pub update: NaiveDateTime,
}

impl From<PotaLogHist> for PotaLogHistRow {
    fn from(l: PotaLogHist) -> Self {
        PotaLogHistRow {
            user_id: l.user_id,
            log_id: l.log_id,
            update: l.update,
        }
    }
}
