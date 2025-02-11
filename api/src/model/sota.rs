use serde::{Deserialize, Serialize};

use common::utils::maidenhead;
use domain::model::event::PagenatedResult;
use domain::model::sota::SOTAReference;
use domain::model::Maidenhead;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateRefRequest {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<CreateRefRequest> for Vec<SOTAReference> {
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
            latitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = value;
        vec![SOTAReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j: Some(summit_name_j),
            city: Some(city),
            city_j: Some(city_j),
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude: Some(longitude),
            latitude: Some(latitude),
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        }]
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRefRequest {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: String,
    pub city: String,
    pub city_j: String,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: String,
    pub valid_to: String,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<UpdateRefRequest> for Vec<SOTAReference> {
    fn from(value: UpdateRefRequest) -> Self {
        let UpdateRefRequest {
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
        let request = SOTAReference {
            summit_code,
            association_name,
            region_name,
            summit_name,
            summit_name_j: Some(summit_name_j),
            city: Some(city),
            city_j: Some(city_j),
            alt_m,
            alt_ft,
            grid_ref1,
            grid_ref2,
            longitude: Some(longitude),
            latitude: Some(latitude),
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        };
        vec![request]
    }
}

#[derive(Debug, Serialize, Default)]
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
    pub maidenhead: Maidenhead,
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
            maidenhead,
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
            maidenhead,
            points,
            bonus_points,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

impl From<(Maidenhead, SOTAReference)> for SOTARefResponse {
    #[allow(unused_variables)]
    fn from((maidenhead, value): (Maidenhead, SOTAReference)) -> Self {
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
            maidenhead,
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
            maidenhead,
            points,
            bonus_points,
            activation_count,
            activation_date,
            activation_call,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagenatedResponse<SOTAReference> {
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
    pub results: Vec<SOTAReference>,
}

impl From<PagenatedResult<SOTAReference>> for PagenatedResponse<SOTARefResponse> {
    fn from(pagenated: PagenatedResult<SOTAReference>) -> Self {
        PagenatedResponse {
            total: pagenated.total,
            limit: pagenated.limit,
            offset: pagenated.offset,
            results: pagenated
                .results
                .into_iter()
                .map(SOTARefResponse::from)
                .collect(),
        }
    }
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
