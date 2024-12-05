use crate::model::sota::{SOTABriefReference, SOTAReference};
use geo_types::Rect;

pub struct CreateRef(pub SOTAReference);

pub struct UpdateRef {
    pub summit_code: String,
    pub summit_name: Option<String>,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: Option<i32>,
    pub longitude: Option<f64>,
    pub lattitude: Option<f64>,
}
pub struct DeleteRef {
    pub summit_code: String,
}

pub struct CreateRefs {
    pub requests: Vec<CreateRef>,
}
pub struct UpdateRefs {
    pub requests: Vec<UpdateRef>,
}

#[derive(Default)]
pub struct SearchRefs {
    pub summit_code: Option<String>,
    pub keyword: Option<String>,
    pub elevation: Option<i32>,
    pub max_results: Option<usize>,
    pub region: Option<Rect>,
}

#[derive(Debug, Default)]
pub struct SearchResults {
    pub results: Option<Vec<SOTAReference>>,
    pub brief_results: Option<Vec<SOTABriefReference>>,
    pub counts: usize,
}

pub struct UploadCSV {}
