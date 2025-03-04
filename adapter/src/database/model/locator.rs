use domain::model::locator::{CenturyCode, MunicipalityCenturyCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize)]
pub enum CenturyCodeImpl {}

#[derive(Debug, FromRow)]
pub struct MunicipalityCenturyCodeRow {
    pub muni_code: i64,
    pub prefecture: String,
    pub municipality: String,
    pub jcc_code: Option<String>,
    pub ward_code: Option<String>,
    pub jcc_text: Option<String>,
    pub jcg_code: Option<String>,
    pub jcg_text: Option<String>,
    pub hamlog_code: Option<String>,
}

impl From<MunicipalityCenturyCode> for MunicipalityCenturyCodeRow {
    fn from(m: MunicipalityCenturyCode) -> Self {
        match m.code {
            CenturyCode::JCC {
                jcc_code,
                ward_code,
                jcc_text,
            } => Self {
                muni_code: m.muni_code as i64,
                prefecture: m.prefecture,
                municipality: m.municipality,
                jcc_code: Some(jcc_code),
                ward_code,
                jcc_text: Some(jcc_text),
                jcg_code: None,
                jcg_text: None,
                hamlog_code: None,
            },
            CenturyCode::JCG {
                jcg_code,
                jcg_text,
                hamlog_code,
            } => Self {
                muni_code: m.muni_code as i64,
                prefecture: m.prefecture,
                municipality: m.municipality,
                jcc_code: None,
                ward_code: None,
                jcc_text: None,
                jcg_code: Some(jcg_code),
                jcg_text: Some(jcg_text),
                hamlog_code,
            },
        }
    }
}

impl From<MunicipalityCenturyCodeRow> for MunicipalityCenturyCode {
    fn from(m: MunicipalityCenturyCodeRow) -> Self {
        if m.jcc_code.is_some() {
            MunicipalityCenturyCode {
                muni_code: m.muni_code as i32,
                prefecture: m.prefecture,
                municipality: m.municipality,
                code: CenturyCode::JCC {
                    jcc_code: m.jcc_code.unwrap(),
                    ward_code: m.ward_code,
                    jcc_text: m.jcc_text.unwrap(),
                },
            }
        } else {
            MunicipalityCenturyCode {
                muni_code: m.muni_code as i32,
                prefecture: m.prefecture,
                municipality: m.municipality,
                code: CenturyCode::JCG {
                    jcg_code: m.jcg_code.unwrap(),
                    jcg_text: m.jcg_text.unwrap(),
                    hamlog_code: m.hamlog_code,
                },
            }
        }
    }
}
