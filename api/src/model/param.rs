use serde::Deserialize;
use std::str::FromStr;

use common::error::AppResult;
use domain::model::{
    event::{FindRef, FindRefBuilder},
    id::LogId,
};

#[derive(Debug, Deserialize)]
pub struct GetParam {
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub dist: Option<f64>,
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub min_elev: Option<i32>,
    pub min_area: Option<i32>,
    pub max_count: Option<u32>,
    pub pota_code: Option<String>,
    pub sota_code: Option<String>,
    pub wwff_code: Option<String>,
    pub user_id: Option<String>,
    pub log_id: Option<String>,
    pub name: Option<String>,
    pub hours_ago: Option<i64>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub muni_code: Option<i32>,
    pub by_call: Option<String>,
    pub by_ref: Option<String>,
    pub pat_ref: Option<String>,
}

pub fn build_findref_query(param: GetParam, mut query: FindRefBuilder) -> AppResult<FindRef> {
    if param.limit.is_some() {
        query = query.limit(param.limit.unwrap());
    }

    if param.offset.is_some() {
        query = query.offset(param.offset.unwrap());
    }

    if param.name.is_some() {
        query = query.name(param.name.unwrap());
    }

    if param.sota_code.is_some() {
        query = query.sota_code(param.sota_code.unwrap());
    }

    if param.pota_code.is_some() {
        query = query.pota_code(param.pota_code.unwrap());
    }

    if param.wwff_code.is_some() {
        query = query.wwff_code(param.wwff_code.unwrap());
    }

    if param.log_id.is_some() {
        if let Ok(log_id) = LogId::from_str(&param.log_id.unwrap()) {
            query = query.log_id(log_id);
        }
    }

    if param.min_area.is_some() {
        query = query.min_area(param.min_area.unwrap());
    }

    if param.min_elev.is_some() {
        query = query.min_elev(param.min_elev.unwrap());
    }

    if param.max_lat.is_some()
        && param.min_lat.is_some()
        && param.max_lon.is_some()
        && param.min_lon.is_some()
    {
        query = query.bbox(
            param.min_lon.unwrap(),
            param.min_lat.unwrap(),
            param.max_lon.unwrap(),
            param.max_lat.unwrap(),
        );
    } else if param.dist.is_some() && param.lon.is_some() && param.lat.is_some() {
        query = query
            .lon(param.lon.unwrap())
            .lat(param.lat.unwrap())
            .dist(param.dist.unwrap());
    }

    Ok(query.build())
}
