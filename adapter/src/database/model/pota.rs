use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use domain::model::id::{LogId, UserId};
use domain::model::pota::{PotaActLog, PotaHuntLog, PotaLogHist, PotaRefLog, PotaReference};
use std::str::FromStr;

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
            longitude: Some(r.longitude),
            latitude: Some(r.latitude),
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
            longitude: r.longitude.unwrap_or_default(),
            latitude: r.latitude.unwrap_or_default(),
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
            longitude: r.longitude.unwrap(),
            latitude: r.latitude.unwrap(),
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
    pub pota_code: String,
    pub first_qso_date: NaiveDate,
    pub attempts: Option<i32>,
    pub activations: Option<i32>,
    pub qsos: i32,
}

impl From<PotaActLog> for PotaLogRow {
    fn from(l: PotaActLog) -> Self {
        PotaLogRow {
            log_id: l.log_id,
            pota_code: l.pota_code,
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
            pota_code: l.pota_code,
            first_qso_date: l.first_qso_date,
            attempts: None,
            activations: None,
            qsos: l.qsos,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PotaLogHistRow {
    pub user_id: Option<UserId>,
    pub log_id: LogId,
    pub log_kind: Option<String>,
    pub update: NaiveDateTime,
}

impl From<PotaLogHist> for PotaLogHistRow {
    fn from(l: PotaLogHist) -> Self {
        PotaLogHistRow {
            user_id: l.user_id,
            log_id: l.log_id,
            log_kind: l.log_kind.map(|k| k.into()),
            update: l.update,
        }
    }
}

impl From<PotaLogHistRow> for PotaLogHist {
    fn from(l: PotaLogHistRow) -> Self {
        PotaLogHist {
            user_id: l.user_id,
            log_id: l.log_id,
            log_kind: l.log_kind.map(|k| k.into()),
            update: l.update,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PotaLegcayLogRow {
    pub uuid: Option<String>,
    pub pota_code: Option<String>,
    pub log_type: Option<i64>,
    pub date: Option<String>,
    pub qso: Option<i64>,
    pub attempt: Option<i64>,
    pub activate: Option<i64>,
}

impl From<PotaLegcayLogRow> for PotaLogRow {
    fn from(value: PotaLegcayLogRow) -> Self {
        let date = value.date.unwrap();
        let mut first_qso_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d");
        if first_qso_date.is_err() {
            first_qso_date = NaiveDate::parse_from_str(&date, "%Y/%m/%d");
        }
        PotaLogRow {
            log_id: LogId::from_str(&value.uuid.unwrap()).unwrap(),
            pota_code: value.pota_code.unwrap(),
            first_qso_date: first_qso_date.unwrap(),
            attempts: (value.attempt.unwrap() != 0).then_some(value.attempt.unwrap() as i32),
            activations: (value.activate.unwrap() != 0).then_some(value.activate.unwrap() as i32),
            qsos: value.qso.unwrap() as i32,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PotaLegcayLogHistRow {
    pub uuid: Option<String>,
    pub time: Option<i64>,
}

impl From<PotaLegcayLogHistRow> for PotaLogHistRow {
    fn from(value: PotaLegcayLogHistRow) -> Self {
        let epoch = Utc.timestamp_opt(value.time.unwrap(), 0);
        let update = match epoch {
            chrono::LocalResult::Single(t) => t.naive_utc(),
            _ => Utc::now().naive_utc(),
        };
        PotaLogHistRow {
            user_id: None,
            log_id: LogId::from_str(&value.uuid.unwrap()).unwrap(),
            log_kind: None,
            update,
        }
    }
}
