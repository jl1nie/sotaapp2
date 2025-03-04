use chrono::NaiveDate;
use serde::Serialize;

use domain::model::geomag::GeomagIndex;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeomagView {
    pub date: NaiveDate,
    pub a_index: i32,
    pub k_index: Vec<i32>,
}

impl From<GeomagIndex> for GeomagView {
    fn from(gi: GeomagIndex) -> GeomagView {
        let GeomagIndex {
            date,
            a_index,
            k_index,
        } = gi;
        GeomagView {
            date,
            a_index,
            k_index: k_index.into_iter().map(|v| v as i32).collect(),
        }
    }
}
