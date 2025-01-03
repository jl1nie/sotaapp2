use chrono::NaiveDate;
use serde::Serialize;

use domain::model::geomag::GeomagIndex;

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GeomagResponse {
    pub date: NaiveDate,
    pub a_index: i32,
    pub k_index: f32,
}

impl From<GeomagIndex> for GeomagResponse {
    fn from(gi: GeomagIndex) -> GeomagResponse {
        let GeomagIndex {
            date,
            a_index,
            k_index,
        } = gi;
        GeomagResponse {
            date,
            a_index,
            k_index,
        }
    }
}
