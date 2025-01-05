#[derive(Debug)]
pub enum CenturyCode {
    JCC {
        jcc_code: String,
        ward_code: Option<String>,
        jcc_text: String,
    },
    JCG {
        jcg_code: String,
        jcg_text: String,
        hamlog_code: Option<String>,
    },
}

#[derive(Debug)]
pub struct MunicipalityCenturyCode {
    pub muni_code: i32,
    pub prefecture: String,
    pub municipality: String,
    pub code: CenturyCode,
}
