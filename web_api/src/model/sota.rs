use domain::model::common::event::UpdateRef;
use domain::model::sota::{SOTARefOptInfo, SOTAReference};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateRefRequest {
    pub summit_code: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

impl From<CreateRefRequest> for UpdateRef<SOTARefOptInfo> {
    fn from(value: CreateRefRequest) -> Self {
        let CreateRefRequest {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
        } = value;
        Self {
            requests: vec![SOTARefOptInfo {
                summit_code,
                summit_name,
                summit_name_j,
                city,
                city_j,
                alt_m,
                longitude,
                latitude,
            }],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub summit_code: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

impl From<UpdateRefRequest> for UpdateRef<SOTARefOptInfo> {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
        } = value;
        let request = SOTARefOptInfo {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
        };
        Self {
            requests: vec![request],
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetParam {
    pub min_lon: Option<f64>,
    pub min_lat: Option<f64>,
    pub max_lon: Option<f64>,
    pub max_lat: Option<f64>,
    pub min_elev: Option<i32>,
    pub min_area: Option<i32>,
    pub ref_id: Option<String>,
    pub name: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTARefResponse {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub points: i32,
    pub bonus_points: i32,
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
            latitude,
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
            association_name,
            region_name,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            latitude,
            points,
            bonus_points,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTARefSearchResponse {
    pub counts: i32,
    pub result: Vec<SOTASearchResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SOTASearchResult {
    pub code: String,
    pub name: String,
    pub name_j: Option<String>,
    pub alt: i32,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub pts: i32,
    pub count: i32,
}

impl From<SOTAReference> for SOTASearchResult {
    fn from(value: SOTAReference) -> Self {
        let SOTAReference {
            summit_code,
            summit_name,
            summit_name_j,
            alt_m,
            longitude,
            latitude,
            points,
            activation_count,
            ..
        } = value;

        Self {
            code: summit_code,
            name: summit_name,
            name_j: summit_name_j,
            alt: alt_m,
            lon: longitude,
            lat: latitude,
            pts: points,
            count: activation_count,
        }
    }
}
