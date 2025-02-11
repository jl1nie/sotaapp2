use csv::ReaderBuilder;
use maidenhead::longlat_to_grid;
use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

pub fn csv_reader<T: DeserializeOwned + std::fmt::Debug>(
    csv: String,
    skip: usize,
) -> AppResult<Vec<T>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
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

const EARTH_RADIUS: f64 = 6371000.0;

pub fn calculate_bounding_box(lat: f64, lon: f64, distance: f64) -> (f64, f64, f64, f64) {
    let lat_radians = lat.to_radians();
    let distance_radians = distance / EARTH_RADIUS;

    let min_lat = lat - distance_radians.to_degrees();
    let max_lat = lat + distance_radians.to_degrees(); // 経度方向の距離を緯度方向に補正する
    let delta_lon = (distance_radians / lat_radians.cos()).to_degrees();
    let min_lon = lon - delta_lon;
    let max_lon = lon + delta_lon;
    (min_lon, min_lat, max_lon, max_lat)
}

pub fn calculate_distance(lat: f64, lon: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat2 - lat).to_radians();
    let dlng = (lon2 - lon).to_radians();

    let lat1 = lat.to_radians();
    let lat2 = lat2.to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

pub fn maidenhead(lon: f64, lat: f64) -> String {
    longlat_to_grid(lon, lat, 8).unwrap_or("--------".to_string())
}
