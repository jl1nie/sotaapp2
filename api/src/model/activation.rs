use serde::Serialize;
use typeshare::typeshare;
use utoipa::ToSchema;

use domain::model::event::GroupBy;

/// アクティベーションビュー
#[derive(Debug, Serialize, ToSchema)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub struct ActivationView<T> {
    pub key: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_view_from_callsign_with_value() {
        let group = GroupBy::Callsign(Some("JA1ABC".to_string()));
        let values = vec![1, 2, 3];

        let view: ActivationView<i32> = (group, values).into();

        assert_eq!(view.key, Some("JA1ABC".to_string()));
        assert_eq!(view.values.len(), 3);
        assert_eq!(view.values[0], 1);
    }

    #[test]
    fn test_activation_view_from_callsign_none() {
        let group = GroupBy::Callsign(None);
        let values = vec!["a", "b"];

        let view: ActivationView<&str> = (group, values).into();

        assert!(view.key.is_none());
        assert_eq!(view.values.len(), 2);
    }

    #[test]
    fn test_activation_view_from_reference_with_value() {
        let group = GroupBy::Reference(Some("JA/TK-001".to_string()));
        let values = vec![10.5, 20.5];

        let view: ActivationView<f64> = (group, values).into();

        assert_eq!(view.key, Some("JA/TK-001".to_string()));
        assert_eq!(view.values.len(), 2);
    }

    #[test]
    fn test_activation_view_from_reference_none() {
        let group = GroupBy::Reference(None);
        let values: Vec<String> = vec![];

        let view: ActivationView<String> = (group, values).into();

        assert!(view.key.is_none());
        assert!(view.values.is_empty());
    }

    #[test]
    fn test_activation_view_json_serialization() {
        let group = GroupBy::Callsign(Some("JA1XYZ".to_string()));
        let values = vec!["spot1".to_string(), "spot2".to_string()];

        let view: ActivationView<String> = (group, values).into();
        let json = serde_json::to_string(&view).unwrap();

        // camelCase形式で出力される
        assert!(json.contains("\"key\":\"JA1XYZ\""));
        assert!(json.contains("\"values\":[\"spot1\",\"spot2\"]"));
    }

    #[test]
    fn test_activation_view_json_serialization_null_key() {
        let group = GroupBy::Callsign(None);
        let values = vec![42];

        let view: ActivationView<i32> = (group, values).into();
        let json = serde_json::to_string(&view).unwrap();

        assert!(json.contains("\"key\":null"));
    }
}
