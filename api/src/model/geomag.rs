use chrono::NaiveDate;
use serde::Serialize;

use domain::model::geomag::GeomagIndex;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeomagResponse {
    pub date: NaiveDate,
    pub a_index: i32,
    pub k_index: Vec<i32>,
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
            k_index: k_index.into_iter().map(|v| v as i32).collect(),
        }
    }
}
