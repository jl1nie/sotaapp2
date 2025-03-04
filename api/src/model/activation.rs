use serde::Serialize;

use domain::model::event::GroupBy;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationView<T> {
    key: Option<String>,
    values: Vec<T>,
}

impl<T> From<(GroupBy, Vec<T>)> for ActivationView<T> {
    fn from(g: (GroupBy, Vec<T>)) -> Self {
        match g.0 {
            GroupBy::Callsign(callsign) => Self {
                key: callsign,
                values: g.1,
            },
            GroupBy::Reference(reference) => Self {
                key: reference,
                values: g.1,
            },
        }
    }
}
