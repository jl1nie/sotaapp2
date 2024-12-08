use crate::model::sota::{SOTARefOptInfo, SOTAReference};
use common::error::{AppError, AppResult};
use csv::ReaderBuilder;
pub struct UploadSOTACSV {
    pub data: String,
}

impl From<UploadSOTACSV> for AppResult<Vec<SOTAReference>> {
    fn from(csv: UploadSOTACSV) -> AppResult<Vec<SOTAReference>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv.data.as_bytes());

        let mut sotalist: Vec<SOTAReference> = Vec::new();
        for result in rdr.deserialize() {
            let req: SOTAReference = result.map_err(AppError::CSVReadError)?;
            sotalist.push(req);
        }
        Ok(sotalist)
    }
}

pub struct UploadSOTAOptCSV {
    pub data: String,
}

impl From<UploadSOTAOptCSV> for AppResult<Vec<SOTARefOptInfo>> {
    fn from(csv: UploadSOTAOptCSV) -> AppResult<Vec<SOTARefOptInfo>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv.data.as_bytes());

        let mut sotalist: Vec<SOTARefOptInfo> = Vec::new();
        for result in rdr.deserialize() {
            let req: SOTARefOptInfo = result.map_err(AppError::CSVReadError)?;
            sotalist.push(req);
        }
        Ok(sotalist)
    }
}
