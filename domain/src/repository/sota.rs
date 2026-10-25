use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

use crate::model::event::{DeleteLog, DeleteRef, FindLog, FindRef, PagenatedResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::model::sota::{
    ComposedPair, DeleteSummitReview, SaveSummitReview, SotaLog, SotaReference, SummitCode,
    SummitEstimateMeta, SummitMap, SummitMapInput, SummitPota, SummitPotaEstimate,
    SummitReviewDetail, SummitReviewSummary,
};

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

    /// 推定を取り込む。`metas` の山頂ごとに推定を消して入れ直し、meta を upsert する。
    /// `full` なら、`metas` にない山頂の推定・meta・地図も消す(目視・確定・履歴は消さない)。
    /// 返り値は、確定していたのに input_rev が変わった山頂の数(要再確認になった数)
    async fn import_summit_estimates(
        &self,
        estimates: Vec<SummitPotaEstimate>,
        metas: Vec<SummitEstimateMeta>,
        full: bool,
    ) -> AppResult<usize>;
    /// 配信の答え(目視があれば目視、なければ推定。all / part だけ)
    async fn find_summit_pota(&self, summit_code: &SummitCode) -> AppResult<Vec<SummitPota>>;
    /// 地図の取り込み用の、山頂ごとの組の答え
    async fn find_composed_pairs(
        &self,
        codes: &[String],
    ) -> AppResult<HashMap<String, Vec<ComposedPair>>>;
    /// 推定の取り込み後の作り直し用の、下地と答え
    async fn find_summit_map_inputs(&self, codes: &[String]) -> AppResult<Vec<SummitMapInput>>;
    /// 下地と配信用の地図を入れる(同じ山頂は上書き)。`base_rev` はその山頂の meta の input_rev
    async fn upsert_summit_maps(
        &self,
        maps: Vec<(SummitMap, String)>,
        now: DateTime<Utc>,
    ) -> AppResult<()>;
    /// 配信用の地図と served_at だけを更新する
    async fn update_served_maps(&self, maps: Vec<SummitMap>, now: DateTime<Utc>) -> AppResult<()>;
    /// 配信用の簡易地図(sota_references にない山頂は返さない)
    async fn find_summit_map(&self, summit_code: &SummitCode) -> AppResult<Option<SummitMap>>;
    /// 山頂コードが prefix で始まる配信用の簡易地図(例: "JA/KN-")
    async fn find_summit_maps(&self, prefix: &str) -> AppResult<Vec<SummitMap>>;
    /// 地域("JA/KN")ごとの、配信用の地図を作り直した最新の日時
    async fn find_map_versions(&self, prefix: &str) -> AppResult<Vec<(String, DateTime<Utc>)>>;

    async fn find_summit_review_list(&self, prefix: &str) -> AppResult<Vec<SummitReviewSummary>>;
    async fn find_summit_review_detail(
        &self,
        summit_code: &SummitCode,
    ) -> AppResult<Option<SummitReviewDetail>>;
    async fn save_summit_review(&self, req: SaveSummitReview) -> AppResult<()>;
    async fn delete_summit_review(&self, req: DeleteSummitReview) -> AppResult<()>;
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
        mock.expect_count_reference().returning(|_| Ok(42));

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

        mock.expect_find_reference().returning(|_| Ok(vec![]));

        let query = FindRefBuilder::default().sota().build();
        let result = mock.find_reference(&query).await;

        assert!(result.unwrap().is_empty());
    }
}
