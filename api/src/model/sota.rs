use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use common::utils::maidenhead;
use domain::model::event::PagenatedResult;
use domain::model::sota::SotaReference;
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

impl From<CreateRefRequest> for Vec<SotaReference> {
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
        vec![SotaReference {
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
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from: NaiveDate::parse_from_str(&valid_from, "%d/%m/%Y").unwrap(),
            valid_to: NaiveDate::parse_from_str(&valid_to, "%d/%m/%Y").unwrap(),
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

impl From<UpdateRefRequest> for Vec<SotaReference> {
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
        let request = SotaReference {
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
            longitude,
            latitude,
            maidenhead: maidenhead(longitude, latitude),
            points,
            bonus_points,
            valid_from: NaiveDate::parse_from_str(&valid_from, "%d/%m/%Y").unwrap(),
            valid_to: NaiveDate::parse_from_str(&valid_to, "%d/%m/%Y").unwrap(),
            activation_count,
            activation_date,
            activation_call,
        };
        vec![request]
    }
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SotaRefView {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
    pub points: i32,
    pub bonus_points: i32,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

impl From<SotaReference> for SotaRefView {
    #[allow(unused_variables)]
    fn from(value: SotaReference) -> Self {
        let SotaReference {
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

impl From<(Maidenhead, SotaReference)> for SotaRefView {
    #[allow(unused_variables)]
    fn from((maidenhead, value): (Maidenhead, SotaReference)) -> Self {
        let SotaReference {
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

impl From<PagenatedResult<SotaReference>> for PagenatedResponse<SotaRefView> {
    fn from(pagenated: PagenatedResult<SotaReference>) -> Self {
        PagenatedResponse {
            total: pagenated.total,
            limit: pagenated.limit,
            offset: pagenated.offset,
            results: pagenated
                .results
                .into_iter()
                .map(SotaRefView::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SotaSearchView {
    pub code: String,
    pub name: String,
    pub name_j: Option<String>,
    pub alt: i32,
    pub lon: f64,
    pub lat: f64,
    pub pts: i32,
    pub count: i32,
}

impl From<SotaReference> for SotaSearchView {
    fn from(value: SotaReference) -> Self {
        let SotaReference {
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
