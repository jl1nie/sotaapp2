use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::activation::{Alert, Spot};
use crate::model::event::{DeleteAct, FindAct};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ActivationRepositry: Send + Sync + Interface {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()>;
    async fn find_alerts(&self, query: &FindAct) -> AppResult<Vec<Alert>>;
    async fn delete_alerts(&self, query: DeleteAct) -> AppResult<()>;
    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()>;
    async fn find_spots(&self, query: &FindAct) -> AppResult<Vec<Spot>>;
    async fn delete_spots(&self, query: DeleteAct) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::event::FindActBuilder;
    use crate::model::AwardProgram;
    use chrono::Utc;

    /// テスト用Alertを生成するヘルパー
    fn make_test_alert(activator: &str, reference: &str) -> Alert {
        Alert {
            program: AwardProgram::SOTA,
            alert_id: 1,
            user_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            location: "Tokyo".to_string(),
            start_time: Utc::now(),
            end_time: None,
            frequencies: "14.280".to_string(),
            comment: Some("Test".to_string()),
            poster: Some(activator.to_string()),
        }
    }

    /// テスト用Spotを生成するヘルパー
    fn make_test_spot(activator: &str, reference: &str) -> Spot {
        Spot {
            program: AwardProgram::SOTA,
            spot_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            spot_time: Utc::now(),
            frequency: "14.280".to_string(),
            mode: "SSB".to_string(),
            spotter: "JA2XYZ".to_string(),
            comment: Some("Test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_mock_find_alerts_empty() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_find_alerts().returning(|_| Ok(vec![]));

        let query = FindActBuilder::default().sota().build();
        let result = mock.find_alerts(&query).await;

        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_find_alerts_with_data() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_find_alerts()
            .returning(|_| Ok(vec![make_test_alert("JA1ABC", "JA/TK-001")]));

        let query = FindActBuilder::default().sota().build();
        let result = mock.find_alerts(&query).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].activator, "JA1ABC");
    }

    #[tokio::test]
    async fn test_mock_find_spots_empty() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_find_spots().returning(|_| Ok(vec![]));

        let query = FindActBuilder::default().sota().build();
        let result = mock.find_spots(&query).await;

        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_find_spots_with_data() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_find_spots()
            .returning(|_| Ok(vec![make_test_spot("JA1ABC", "JA/TK-001")]));

        let query = FindActBuilder::default().sota().build();
        let result = mock.find_spots(&query).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].activator, "JA1ABC");
        assert_eq!(result[0].mode, "SSB");
    }

    #[tokio::test]
    async fn test_mock_update_alerts() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_update_alerts().times(1).returning(|_| Ok(()));

        let alerts = vec![make_test_alert("JA1ABC", "JA/TK-001")];
        let result = mock.update_alerts(alerts).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_update_spots() {
        let mut mock = MockActivationRepositry::new();

        mock.expect_update_spots().times(1).returning(|_| Ok(()));

        let spots = vec![make_test_spot("JA1ABC", "JA/TK-001")];
        let result = mock.update_spots(spots).await;

        assert!(result.is_ok());
    }
}
