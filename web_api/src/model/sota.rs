use application::model::sota::{
    event::{CreateRef, UpdateRef},
    SOTABriefReference, SOTAReference,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateRefRequest {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<CreateRefRequest> for CreateRef {
    fn from(value: CreateRefRequest) -> Self {
        let CreateRefRequest {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            lattitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;
        Self(SOTAReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            lattitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub summit_code: String,
    pub summit_name: Option<String>,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: Option<i32>,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
}

impl From<UpdateRefRequest> for UpdateRef {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            lattitude,
        } = value;
        Self {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            lattitude,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetParam {
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub elevation: Option<i32>,
    pub key: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTARefResponse {
    pub summit_code: String,
    pub association_name: Option<String>,
    pub region_name: Option<String>,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: Option<i32>,
    pub grid_ref1: Option<String>,
    pub grid_ref2: Option<String>,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
    pub points: i32,
    pub bonus_points: Option<i32>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SOTAReference> for SOTARefResponse {
    #[allow(unused_variables)]
    fn from(value: SOTAReference) -> Self {
        let SOTAReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude,
            lattitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;

        Self {
            summit_code,
            association_name: None,
            region_name: None,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            alt_ft: None,
            grid_ref1: None,
            grid_ref2: None,
            longitude,
            lattitude,
            points,
            bonus_points: None,
            valid_from: None,
            valid_to: None,
            activation_count,
            activation_date: None,
            activation_call: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTARefShortResponse {
    pub summit_code: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub alt_m: i32,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
    pub points: i32,
}

impl From<SOTABriefReference> for SOTARefShortResponse {
    fn from(value: SOTABriefReference) -> Self {
        let SOTABriefReference {
            summit_code,
            summit_name,
            summit_name_j,
            alt_m,
            longitude,
            lattitude,
            points,
            ..
        } = value;

        Self {
            summit_code,
            summit_name,
            summit_name_j,
            alt_m,
            longitude,
            lattitude,
            points,
        }
    }
}
