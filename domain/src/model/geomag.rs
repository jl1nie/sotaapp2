use chrono::NaiveDate;
#[derive(Debug, Clone)]
pub struct GeomagIndex {
    pub date: NaiveDate,
    pub a_index: i32,
    pub k_index: f32,
}
