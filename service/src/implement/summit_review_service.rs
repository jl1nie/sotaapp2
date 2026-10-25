use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use common::error::{AppError, AppResult};
use domain::model::sota::{DeleteSummitReview, ParkCoverage, SaveSummitReview, SummitCode};
use domain::repository::sota::SotaRepository;
use shaku::Component;

use crate::model::summit_review::{
    build_served_map, composed_for_map, summarize, validate_answers, ConfirmSummit, ReviewedDetail,
    StatusFilter, SummitReviewList,
};
use crate::services::SummitReviewService;

#[derive(Component)]
#[shaku(interface = SummitReviewService)]
pub struct SummitReviewServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
}

impl SummitReviewServiceImpl {
    async fn find_detail(&self, summit_code: &str) -> AppResult<ReviewedDetail> {
        self.sota_repo
            .find_summit_review_detail(&SummitCode::new(summit_code.to_string()))
            .await?
            .map(ReviewedDetail::new)
            .ok_or_else(|| AppError::EntityNotFound(format!("{summit_code} が見つかりません")))
    }
}

#[async_trait]
impl SummitReviewService for SummitReviewServiceImpl {
    async fn list_summits(
        &self,
        prefix: &str,
        filter: Option<StatusFilter>,
        q: Option<&str>,
    ) -> AppResult<SummitReviewList> {
        let list = self.sota_repo.find_summit_review_list(prefix).await?;
        Ok(summarize(list, filter, q))
    }

    async fn show_summit(&self, summit_code: &str) -> AppResult<ReviewedDetail> {
        self.find_detail(summit_code).await
    }

    async fn confirm_summit(&self, req: ConfirmSummit) -> AppResult<ReviewedDetail> {
        // 1. 詳細を読む(なければ 404)
        let current = self.find_detail(&req.summit_code).await?.detail;
        // 2. 答えの組が今の推定の組とぴったり一致すること
        validate_answers(&current.pairs, &req.answers)?;
        // 3. 見ている間に推定が取り込み直されていないこと(保存の tx の中でも確かめる)
        let current_rev = current
            .meta
            .as_ref()
            .map(|m| m.input_rev.as_str())
            .unwrap_or("");
        if current_rev != req.input_rev {
            return Err(AppError::Conflict(
                "推定が取り込み直されました。読み直してください".to_string(),
            ));
        }
        // 4. 答えで配信用の地図を作る(下地があれば)
        let served_geojson = match &current.base_geojson {
            Some(base) => {
                let pairs = composed_for_map(&current.pairs, |p| {
                    req.answers
                        .iter()
                        .find(|a| {
                            a.muni_key == p.estimate.muni_key && a.pota_code == p.estimate.pota_code
                        })
                        .map(|a| a.coverage)
                        .unwrap_or(ParkCoverage::None)
                });
                Some(build_served_map(base, &pairs)?)
            }
            None => None,
        };
        // 5. 保存
        self.sota_repo
            .save_summit_review(SaveSummitReview {
                summit_code: req.summit_code.clone(),
                expected_input_rev: req.input_rev,
                answers: req.answers,
                reviewed_by: req.reviewed_by,
                note: req.note,
                now: Utc::now(),
                served_geojson,
            })
            .await?;
        // 6. 保存後の詳細
        self.find_detail(&req.summit_code).await
    }

    async fn cancel_review(
        &self,
        summit_code: &str,
        changed_by: &str,
    ) -> AppResult<ReviewedDetail> {
        let current = self.find_detail(summit_code).await?.detail;
        // 目視を外すので、推定の答えで配信用の地図を作り直す
        let served_geojson = match &current.base_geojson {
            Some(base) => {
                let pairs = composed_for_map(&current.pairs, |p| p.estimate.coverage);
                Some(build_served_map(base, &pairs)?)
            }
            None => None,
        };
        self.sota_repo
            .delete_summit_review(DeleteSummitReview {
                summit_code: summit_code.to_string(),
                changed_by: changed_by.to_string(),
                now: Utc::now(),
                served_geojson,
            })
            .await?;
        self.find_detail(summit_code).await
    }
}
