use chrono::NaiveDate;
#[derive(Debug, Clone, Default)]
pub struct GeomagIndex {
    pub date: NaiveDate,
    pub a_index: i32,
    pub k_index: Vec<f32>,
}
