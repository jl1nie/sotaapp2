use crate::model::sota::SOTAReference;

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
    pub refrences: Vec<SOTAReference>,
}
pub struct UpdateRefs {
    pub refrences: Vec<UpdateRef>,
}
