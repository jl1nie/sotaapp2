use serde::Serialize;

use domain::model::common::event::GroupBy;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupByResponse {
    callsign: Option<String>,
    reference: Option<String>,
}

impl From<GroupBy> for GroupByResponse {
    fn from(g: GroupBy) -> Self {
        match g {
            GroupBy::Callsign(callsign) => Self {
                callsign,
                reference: None,
            },
            GroupBy::Reference(reference) => Self {
                callsign: None,
                reference,
            },
        }
    }
}
