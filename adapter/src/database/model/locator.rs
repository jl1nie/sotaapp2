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
        // jcc_codeとjcc_textは同時にSome/Noneのはず（DB制約）
        if let (Some(jcc_code), Some(jcc_text)) = (m.jcc_code, m.jcc_text) {
            MunicipalityCenturyCode {
                muni_code: m.muni_code as i32,
                prefecture: m.prefecture,
                municipality: m.municipality,
                code: CenturyCode::JCC {
                    jcc_code,
                    ward_code: m.ward_code,
                    jcc_text,
                },
            }
        } else if let (Some(jcg_code), Some(jcg_text)) = (m.jcg_code, m.jcg_text) {
            // jcg_codeとjcg_textは同時にSome/Noneのはず（DB制約）
            MunicipalityCenturyCode {
                muni_code: m.muni_code as i32,
                prefecture: m.prefecture,
                municipality: m.municipality,
                code: CenturyCode::JCG {
                    jcg_code,
                    jcg_text,
                    hamlog_code: m.hamlog_code,
                },
            }
        } else {
            // DB制約上、JCCかJCGのいずれかは必ず存在するはず
            // 万が一の場合は空のJCGとして扱う
            MunicipalityCenturyCode {
                muni_code: m.muni_code as i32,
                prefecture: m.prefecture,
                municipality: m.municipality,
                code: CenturyCode::JCG {
                    jcg_code: String::new(),
                    jcg_text: String::new(),
                    hamlog_code: None,
                },
            }
        }
    }
}
