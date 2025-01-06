use serde::Serialize;

use super::pota::POTASearchResult;
use super::sota::SOTASearchResult;
use domain::model::common::event::FindResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerachFullResponse {
    pub sota: Option<Vec<SOTASearchResult>>,
    pub pota: Option<Vec<POTASearchResult>>,
}

impl From<FindResult> for SerachFullResponse {
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
pub struct SearchBriefResponse {
    pub count: u32,
    pub candidates: Vec<(String, String, String)>,
}

impl From<FindResult> for SearchBriefResponse {
    fn from(FindResult { sota, pota }: FindResult) -> Self {
        let mut res = vec![];

        if let Some(sota) = sota {
            sota.iter().for_each(|r| {
                res.push((
                    r.summit_code.clone(),
                    r.summit_name.clone(),
                    r.summit_name_j.clone().unwrap_or("".to_string()),
                ))
            });
        };

        if let Some(pota) = pota {
            pota.into_iter().for_each(|r| {
                res.push((
                    r.pota_code.clone(),
                    r.park_name.clone(),
                    r.park_name_j.clone(),
                ))
            });
        };

        Self {
            count: res.len() as u32,
            candidates: res,
        }
    }
}
