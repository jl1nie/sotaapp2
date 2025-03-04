use chrono::{DateTime, NaiveDate, Utc};
use domain::model::Maidenhead;
use serde::{Deserialize, Serialize};

use common::utils::maidenhead;
use domain::model::pota::{PotaLogHist, PotaRefLog, PotaReference};
use domain::model::{event::PagenatedResult, id::UserId};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRefRequest {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

impl From<CreateRefRequest> for Vec<PotaReference> {
    fn from(value: CreateRefRequest) -> Self {
        let CreateRefRequest {
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
        vec![PotaReference {
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
            maidenhead: maidenhead(longitude.unwrap_or_default(), latitude.unwrap_or_default()),
            update,
        }]
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagenatedResponse<PotaReference> {
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
    pub results: Vec<PotaReference>,
}

impl From<PagenatedResult<PotaReference>> for PagenatedResponse<PotaRefView> {
    fn from(pagenated: PagenatedResult<PotaReference>) -> Self {
        PagenatedResponse {
            total: pagenated.total,
            limit: pagenated.limit,
            offset: pagenated.offset,
            results: pagenated
                .results
                .into_iter()
                .map(PotaRefView::from)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

impl From<UpdateRefRequest> for Vec<PotaReference> {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
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
        vec![PotaReference {
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
            maidenhead: maidenhead(longitude.unwrap_or_default(), latitude.unwrap_or_default()),
            update,
        }]
    }
}

#[derive(Debug)]
pub struct PotaActivatorLog {
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
pub struct PotaHunterLog {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotaRefView {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub maidenhead: Maidenhead,
}

impl From<PotaReference> for PotaRefView {
    fn from(pota: PotaReference) -> Self {
        PotaRefView {
            pota_code: pota.pota_code,
            wwff_code: pota.wwff_code,
            park_name: pota.park_name,
            park_name_j: pota.park_name_j,
            park_location: pota.park_location,
            park_locid: pota.park_locid,
            park_type: pota.park_type,
            park_inactive: pota.park_inactive,
            park_area: pota.park_area,
            longitude: pota.longitude,
            latitude: pota.latitude,
            maidenhead: pota.maidenhead,
        }
    }
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PotaRefLogView {
    pub pota_code: String,
    pub wwff_code: String,
    pub park_name: String,
    pub park_name_j: String,
    pub park_location: String,
    pub park_locid: String,
    pub park_type: String,
    pub park_inactive: bool,
    pub park_area: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub maidenhead: Maidenhead,
    pub attempts: Option<i32>,
    pub activations: Option<i32>,
    pub first_qso_date: Option<NaiveDate>,
    pub qsos: Option<i32>,
}

impl From<PotaRefLog> for PotaRefLogView {
    fn from(pota: PotaRefLog) -> Self {
        PotaRefLogView {
            pota_code: pota.pota_code,
            wwff_code: pota.wwff_code,
            park_name: pota.park_name,
            park_name_j: pota.park_name_j,
            park_location: pota.park_location,
            park_locid: pota.park_locid,
            park_type: pota.park_type,
            park_inactive: pota.park_inactive,
            park_area: pota.park_area,
            longitude: pota.longitude,
            latitude: pota.latitude,
            maidenhead: pota.maidenhead,
            attempts: pota.attempts,
            activations: pota.activations,
            first_qso_date: pota.first_qso_date,
            qsos: pota.qsos,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotaSearchView {
    pub pota: String,
    pub wwff: String,
    pub name: String,
    pub name_j: String,
    pub locid: Vec<String>,
    pub area: i32,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub atmpt: Option<i32>,
    pub act: Option<i32>,
    pub date: Option<NaiveDate>,
    pub qsos: Option<i32>,
}

impl From<PotaRefLog> for PotaSearchView {
    fn from(pota: PotaRefLog) -> Self {
        let locid: Vec<String> = pota
            .park_locid
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        PotaSearchView {
            pota: pota.pota_code,
            wwff: pota.wwff_code,
            name: pota.park_name,
            name_j: pota.park_name_j,
            locid,
            area: pota.park_area,
            lon: pota.longitude,
            lat: pota.latitude,
            atmpt: pota.attempts,
            act: pota.activations,
            date: pota.first_qso_date,
            qsos: pota.qsos,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotaLogHistView {
    pub log_id: String,
    pub log_kind: String,
    pub last_update: NaiveDate,
}

impl From<PotaLogHist> for PotaLogHistView {
    fn from(log: PotaLogHist) -> Self {
        let log_kind = match log.log_kind {
            Some(kind) => kind.into(),
            None => "none".to_string(),
        };

        PotaLogHistView {
            log_id: log.log_id.into(),
            log_kind,
            last_update: log.update.date(),
        }
    }
}
