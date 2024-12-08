use crate::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference};
use common::error::{AppError, AppResult};
use csv::ReaderBuilder;
pub struct UploadPOTACSV {
    pub data: String,
}

impl From<UploadPOTACSV> for AppResult<Vec<POTAReference>> {
    fn from(csv: UploadPOTACSV) -> AppResult<Vec<POTAReference>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv.data.as_bytes());

        let mut potalist: Vec<POTAReference> = Vec::new();
        for result in rdr.deserialize() {
            let req: POTAReference = result.map_err(AppError::CSVReadError)?;
            potalist.push(req);
        }
        Ok(potalist)
    }
}

pub struct UploadActivatorCSV {
    pub data: String,
}

impl From<UploadActivatorCSV> for AppResult<Vec<POTAActivatorLog>> {
    fn from(csv: UploadActivatorCSV) -> AppResult<Vec<POTAActivatorLog>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv.data.as_bytes());

        let mut potalog: Vec<POTAActivatorLog> = Vec::new();
        for result in rdr.deserialize() {
            let req: POTAActivatorLog = result.map_err(AppError::CSVReadError)?;
            potalog.push(req);
        }
        Ok(potalog)
    }
}

pub struct UploadHunterCSV {
    pub data: String,
}

impl From<UploadHunterCSV> for AppResult<Vec<POTAHunterLog>> {
    fn from(csv: UploadHunterCSV) -> AppResult<Vec<POTAHunterLog>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv.data.as_bytes());

        let mut potalog: Vec<POTAHunterLog> = Vec::new();
        for result in rdr.deserialize() {
            let req: POTAHunterLog = result.map_err(AppError::CSVReadError)?;
            potalog.push(req);
        }
        Ok(potalog)
    }
}
