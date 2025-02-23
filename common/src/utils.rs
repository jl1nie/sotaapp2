use csv::ReaderBuilder;
use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use maidenhead::longlat_to_grid;
use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

pub fn csv_reader<T: DeserializeOwned + std::fmt::Debug>(
    csv: String,
    has_headers: bool,
    skip: usize,
) -> AppResult<Vec<T>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .flexible(true)
        .from_reader(csv.as_bytes());

    let mut reflist: Vec<T> = Vec::new();
    for result in rdr.records().skip(skip) {
        let req: T = result
            .map_err(AppError::CSVReadError)?
            .deserialize(None)
            .map_err(AppError::CSVReadError)?;
        reflist.push(req);
    }
    Ok(reflist)
}

pub fn call_to_operator(callsign: &str) -> String {
    let callsign = callsign.trim_end().to_string();
    let parts: Vec<_> = callsign.split("/").collect();
    match parts.len() {
        1 => parts[0].to_string(),
        2 => parts[0].to_string(),
        3 => parts[1].to_string(),
        _ => callsign,
    }
}

pub fn calculate_bounding_box(lat: f64, lon: f64, distance: f64) -> (f64, f64, f64, f64) {
    let g = Geodesic::wgs84();

    let (max_lat, max_lon, _, _) = g.direct(lat, lon, 45.0, distance);
    let (min_lat, min_lon, _, _) = g.direct(lat, lon, 225.0, distance);

    (min_lat, min_lon, max_lat, max_lon)
}

pub fn calculate_distance(lat: f64, lon: f64, lat2: f64, lon2: f64) -> f64 {
    let g = Geodesic::wgs84();

    g.inverse(lat, lon, lat2, lon2)
}

pub fn maidenhead(lon: f64, lat: f64) -> String {
    longlat_to_grid(lon, lat, 8).unwrap_or("--------".to_string())
}
