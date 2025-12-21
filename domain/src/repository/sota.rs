use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::event::{DeleteLog, DeleteRef, FindLog, FindRef, PagenatedResult};
use crate::model::sota::{SotaLog, SotaReference, SummitCode};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait SotaRepository: Send + Sync + Interface {
    async fn count_reference(&self, query: &FindRef) -> AppResult<i64>;
    async fn find_reference(&self, query: &FindRef) -> AppResult<Vec<SotaReference>>;

    async fn create_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn show_reference(&self, query: &FindRef) -> AppResult<SotaReference>;
    async fn show_all_references(
        &self,
        query: &FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>>;
    async fn update_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn upsert_reference(&self, references: Vec<SotaReference>) -> AppResult<()>;
    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()>;

    async fn upload_log(&self, logs: Vec<SotaLog>) -> AppResult<()>;
    async fn find_log(&self, query: &FindLog) -> AppResult<Vec<SotaLog>>;
    async fn delete_log(&self, query: DeleteLog) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::event::FindRefBuilder;

    /// MockSotaRepositoryが正しく生成できることを確認
    #[test]
    fn test_mock_sota_repository_creation() {
        let mock = MockSotaRepository::new();
        drop(mock);
    }

    /// expectを使った基本的なモック設定サンプル
    #[tokio::test]
    async fn test_mock_sota_repository_count_reference() {
        let mut mock = MockSotaRepository::new();

        // count_referenceが呼ばれたら42を返すように設定
        mock.expect_count_reference()
            .returning(|_| Ok(42));

        let query = FindRefBuilder::default().sota().build();
        let result = mock.count_reference(&query).await;

        assert_eq!(result.unwrap(), 42);
    }

    /// timesで呼び出し回数を検証するサンプル
    #[tokio::test]
    async fn test_mock_with_times() {
        let mut mock = MockSotaRepository::new();

        mock.expect_count_reference()
            .times(2)
            .returning(|_| Ok(100));

        let query = FindRefBuilder::default().sota().build();

        let _ = mock.count_reference(&query).await;
        let result = mock.count_reference(&query).await;

        assert_eq!(result.unwrap(), 100);
    }

    /// find_referenceのモックサンプル（空ベクタを返す）
    #[tokio::test]
    async fn test_mock_find_reference_empty() {
        let mut mock = MockSotaRepository::new();

        mock.expect_find_reference()
            .returning(|_| Ok(vec![]));

        let query = FindRefBuilder::default().sota().build();
        let result = mock.find_reference(&query).await;

        assert!(result.unwrap().is_empty());
    }
}
