pub mod event;

#[derive(Debug)]
pub struct SOTAReference {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: i32,
    pub gird_ref2: i32,
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
