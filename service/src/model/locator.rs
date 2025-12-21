use domain::model::locator::{CenturyCode, MunicipalityCenturyCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MuniCSVFile {
    pub muni_code: i32,
    pub prefecture: String,
    pub municipality: String,
    pub jcc_code: Option<String>,
    pub ward_code: Option<String>,
    pub jcc_text: Option<String>,
    pub jcg_code: Option<String>,
    pub jcg_text: Option<String>,
    pub hamlog_code: Option<String>,
}

impl From<MuniCSVFile> for MunicipalityCenturyCode {
    fn from(csv: MuniCSVFile) -> MunicipalityCenturyCode {
        let MuniCSVFile {
            muni_code,
            prefecture,
            municipality,
            jcc_code,
            ward_code,
            jcc_text,
            jcg_code,
            jcg_text,
            hamlog_code,
        } = csv;
        let code = match jcc_code {
            Some(jcc_code) => CenturyCode::JCC {
                jcc_code,
                ward_code,
                jcc_text: jcc_text.unwrap_or_default(),
            },
            _ => CenturyCode::JCG {
                jcg_code: jcg_code.unwrap_or_default(),
                jcg_text: jcg_text.unwrap_or_default(),
                hamlog_code,
            },
        };
        MunicipalityCenturyCode {
            muni_code,
            prefecture,
            municipality,
            code,
        }
    }
}
pub struct UploadMuniCSV {
    pub data: String,
}
