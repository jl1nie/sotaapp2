use common::error::{AppError, AppResult};
use csv::ReaderBuilder;
use serde::de::DeserializeOwned;

pub fn csv_reader<T: DeserializeOwned>(csv: String) -> AppResult<Vec<T>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv.as_bytes());

    let mut reflist: Vec<T> = Vec::new();
    for result in rdr.deserialize() {
        let req: T = result.map_err(AppError::CSVReadError)?;
        reflist.push(req);
    }
    Ok(reflist)
}
