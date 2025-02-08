use serde::Serialize;

use super::pota::{POTARefResponseWithLog, POTASearchResult};
use super::sota::{SOTARefResponse, SOTASearchResult};
use domain::model::event::FindResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub sota: Option<Vec<SOTASearchResult>>,
    pub pota: Option<Vec<POTASearchResult>>,
}
impl From<FindResult> for SearchResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        Self {
            sota: if let Some(sota) = sota {
                let res = sota.into_iter().map(SOTASearchResult::from).collect();
                Some(res)
            } else {
                None
            },
            pota: if let Some(pota) = pota {
                let res = pota.into_iter().map(POTASearchResult::from).collect();
                Some(res)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFullResponse {
    pub sota: Option<Vec<SOTARefResponse>>,
    pub pota: Option<Vec<POTARefResponseWithLog>>,
}
impl From<FindResult> for SearchFullResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        Self {
            sota: if let Some(sota) = sota {
                let res = sota.into_iter().map(SOTARefResponse::from).collect();
                Some(res)
            } else {
                None
            },
            pota: if let Some(pota) = pota {
                let res = pota.into_iter().map(POTARefResponseWithLog::from).collect();
                Some(res)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchBriefResponse {
    pub count: u32,
    pub candidates: Vec<SearchBriefData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchBriefData {
    pub code: String,
    pub lon: f64,
    pub lat: f64,
    pub name: String,
    pub name_j: String,
}

impl From<FindResult> for SearchBriefResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        let mut res = vec![];

        if let Some(sota) = sota {
            sota.iter().for_each(|r| {
                res.push(SearchBriefData {
                    code: r.summit_code.clone(),
                    lon: r.longitude.unwrap_or_default(),
                    lat: r.latitude.unwrap_or_default(),
                    name: r.summit_name.clone(),
                    name_j: r.summit_name_j.clone().unwrap_or_default(),
                })
            })
        };

        if let Some(pota) = pota {
            pota.into_iter().for_each(|r| {
                res.push(SearchBriefData {
                    code: r.pota_code,
                    lon: r.longitude.unwrap_or_default(),
                    lat: r.latitude.unwrap_or_default(),
                    name: r.park_name,
                    name_j: r.park_name_j,
                })
            });
        };
        Self {
            count: res.len() as u32,
            candidates: res,
        }
    }
}
