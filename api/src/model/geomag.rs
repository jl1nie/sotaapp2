use serde::Serialize;
use typeshare::typeshare;

use domain::model::geomag::GeomagIndex;

#[derive(Debug, Serialize)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct GeomagView {
    pub date: String,
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
            date: date.to_string(),
            a_index,
            k_index: k_index.into_iter().map(|v| v as i32).collect(),
        }
    }
}
