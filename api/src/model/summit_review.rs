//! 見直しの API の入出力(設計 9 章)。JSON は camelCase
use chrono::{DateTime, Utc};
use domain::model::admin_user::AdminUser;
use domain::model::sota::{DataVersions, ParkCoverage, SummitPotaVerdict, SummitReview};
use serde::{Deserialize, Serialize};
use service::model::sota::SummitPotaImportResult;
use service::model::summit_review::{ReviewStatus, ReviewedDetail, SummitReviewList};

use crate::handler::auth::{AdminIdentity, AuthMethod};

fn iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn cov(c: ParkCoverage) -> &'static str {
    c.as_str()
}

fn status_str(s: ReviewStatus) -> &'static str {
    match s {
        ReviewStatus::NoMap => "no_map",
        ReviewStatus::Unreviewed => "unreviewed",
        ReviewStatus::NeedsCheck => "needs_check",
        ReviewStatus::Rereview => "rereview",
        ReviewStatus::Confirmed => "confirmed",
    }
}

// ---- 一覧 ----

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CountsView {
    pub no_map: usize,
    pub unreviewed: usize,
    pub needs_check: usize,
    pub rereview: usize,
    pub confirmed: usize,
    pub stale: usize,
    pub map_mismatch: usize,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummitRowView {
    pub summit_code: String,
    pub summit_name: String,
    pub pairs: i64,
    pub all_pairs: i64,
    pub part_pairs: i64,
    pub none_pairs: i64,
    pub flagged_pairs: i64,
    pub status: &'static str,
    pub stale: bool,
    pub map_mismatch: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummitReviewListView {
    pub counts: CountsView,
    pub summits: Vec<SummitRowView>,
}

impl From<SummitReviewList> for SummitReviewListView {
    fn from(l: SummitReviewList) -> Self {
        let c = l.counts;
        Self {
            counts: CountsView {
                no_map: c.no_map,
                unreviewed: c.unreviewed,
                needs_check: c.needs_check,
                rereview: c.rereview,
                confirmed: c.confirmed,
                stale: c.stale,
                map_mismatch: c.map_mismatch,
            },
            summits: l
                .rows
                .into_iter()
                .map(|r| SummitRowView {
                    summit_code: r.summary.summit_code,
                    summit_name: r.summary.summit_name,
                    pairs: r.summary.pairs,
                    all_pairs: r.summary.all_pairs,
                    part_pairs: r.summary.part_pairs,
                    none_pairs: r.summary.none_pairs,
                    flagged_pairs: r.summary.flagged_pairs,
                    status: status_str(r.status),
                    stale: r.stale,
                    map_mismatch: r.map_mismatch,
                    reviewed_by: r.summary.review.as_ref().map(|v| v.reviewed_by.clone()),
                    reviewed_at: r.summary.review.as_ref().map(|v| iso(v.reviewed_at)),
                })
                .collect(),
        }
    }
}

// ---- 詳細 ----

#[derive(Debug, Serialize, PartialEq)]
pub struct VersionsView {
    pub park: String,
    pub muni: String,
    pub munitable: String,
}

impl From<DataVersions> for VersionsView {
    fn from(v: DataVersions) -> Self {
        Self {
            park: v.park,
            muni: v.muni,
            munitable: v.munitable,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairView {
    pub muni_key: String,
    pub muni_name: String,
    /// city_j に `<muni_key>` がある
    pub in_city_j: bool,
    pub pota_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub park_name: Option<String>,
    pub park_inactive: bool,
    pub estimate: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_px: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dist_m: Option<f64>,
    pub noise: bool,
    pub shallow: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<&'static str>,
    pub needs_review: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<&'static str>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerdictView {
    pub muni_key: String,
    pub pota_code: String,
    pub coverage: &'static str,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewView {
    pub input_rev: String,
    pub reviewed_by: String,
    pub reviewed_at: String,
    pub versions: VersionsView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl From<SummitReview> for ReviewView {
    fn from(r: SummitReview) -> Self {
        Self {
            input_rev: r.input_rev,
            reviewed_by: r.reviewed_by,
            reviewed_at: iso(r.reviewed_at),
            versions: r.versions.into(),
            note: r.note,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub muni_key: String,
    pub pota_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<&'static str>,
    pub changed_by: String,
    pub changed_at: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummitReviewDetailView {
    pub summit_code: String,
    pub summit_name: String,
    pub alt_m: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_j: Option<String>,
    pub activation_count: i64,
    pub status: &'static str,
    pub stale: bool,
    pub map_mismatch: bool,
    pub input_rev: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<VersionsView>,
    pub reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed_at: Option<String>,
    pub pairs: Vec<PairView>,
    /// 今の候補にない組の目視(参考)
    pub previous_verdicts: Vec<VerdictView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<ReviewView>,
    /// 下地の GeoJSON(文字列ではなく JSON で)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_map: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub served_at: Option<String>,
    pub log: Vec<LogView>,
}

impl From<ReviewedDetail> for SummitReviewDetailView {
    fn from(r: ReviewedDetail) -> Self {
        let d = r.detail;
        let verdict_of = |key: &str, code: &str| {
            d.verdicts
                .iter()
                .find(|v| v.muni_key == key && v.pota_code == code)
                .map(|v| cov(v.coverage))
        };
        let pairs: Vec<PairView> = d
            .pairs
            .iter()
            .map(|p| {
                let e = &p.estimate;
                PairView {
                    in_city_j: d
                        .city_j
                        .as_deref()
                        .is_some_and(|c| c.contains(&format!("<{}>", e.muni_key))),
                    muni_key: e.muni_key.clone(),
                    muni_name: e.muni_name.clone(),
                    pota_code: e.pota_code.clone(),
                    park_name: p.park_name.clone(),
                    park_inactive: p.park_inactive,
                    estimate: cov(e.coverage),
                    pct: e.pct,
                    px: e.px,
                    tot_px: e.tot_px,
                    dist_m: e.dist_m,
                    noise: e.noise,
                    shallow: e.shallow,
                    hint: e.hint.map(cov),
                    needs_review: e.needs_review,
                    note: e.note.clone(),
                    verdict: verdict_of(&e.muni_key, &e.pota_code),
                }
            })
            .collect();
        let previous_verdicts = d
            .verdicts
            .iter()
            .filter(|v| {
                !d.pairs.iter().any(|p| {
                    p.estimate.muni_key == v.muni_key && p.estimate.pota_code == v.pota_code
                })
            })
            .map(|v| VerdictView {
                muni_key: v.muni_key.clone(),
                pota_code: v.pota_code.clone(),
                coverage: cov(v.coverage),
            })
            .collect();
        let meta = d.meta;
        Self {
            summit_code: d.summit_code,
            summit_name: d.summit_name,
            alt_m: d.alt_m,
            latitude: d.latitude,
            longitude: d.longitude,
            city_j: d.city_j,
            activation_count: d.activation_count,
            status: status_str(r.status),
            stale: r.stale,
            map_mismatch: r.map_mismatch,
            input_rev: meta
                .as_ref()
                .map(|m| m.input_rev.clone())
                .unwrap_or_default(),
            versions: meta.as_ref().map(|m| m.versions.clone().into()),
            reasons: meta.as_ref().map(|m| m.reasons.clone()).unwrap_or_default(),
            change_note: meta.as_ref().and_then(|m| m.change_note.clone()),
            computed_at: meta.as_ref().map(|m| iso(m.computed_at)),
            pairs,
            previous_verdicts,
            review: d.review.map(Into::into),
            base_map: d
                .base_geojson
                .as_deref()
                .and_then(|g| serde_json::from_str(g).ok()),
            base_rev: d.base_rev,
            served_at: d.served_at.map(iso),
            log: d
                .log
                .into_iter()
                .map(|l| LogView {
                    muni_key: l.muni_key,
                    pota_code: l.pota_code,
                    before: l.before.map(cov),
                    after: l.after.map(cov),
                    changed_by: l.changed_by,
                    changed_at: iso(l.changed_at),
                })
                .collect(),
        }
    }
}

// ---- 確定の要求 ----

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CoverageDto {
    All,
    Part,
    None,
}

impl From<CoverageDto> for ParkCoverage {
    fn from(c: CoverageDto) -> Self {
        match c {
            CoverageDto::All => ParkCoverage::All,
            CoverageDto::Part => ParkCoverage::Part,
            CoverageDto::None => ParkCoverage::None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDto {
    pub muni_key: String,
    pub pota_code: String,
    pub coverage: CoverageDto,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRequest {
    pub input_rev: String,
    pub answers: Vec<AnswerDto>,
    #[serde(default)]
    pub note: Option<String>,
}

impl ConfirmRequest {
    pub fn verdicts(&self) -> Vec<SummitPotaVerdict> {
        self.answers
            .iter()
            .map(|a| SummitPotaVerdict {
                muni_key: a.muni_key.clone(),
                pota_code: a.pota_code.clone(),
                coverage: a.coverage.into(),
            })
            .collect()
    }
}

// ---- 地図の版・取り込み・管理者 ----

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MapVersionView {
    pub region: String,
    pub updated_at: String,
}

impl MapVersionView {
    pub fn new(region: String, at: DateTime<Utc>) -> Self {
        Self {
            region,
            updated_at: iso(at),
        }
    }
}

/// 推定の取り込みの返り値(`ImportResult` に山頂の数と要再確認の数を足したもの)
#[derive(Debug, Serialize, PartialEq)]
pub struct PotaImportResultView {
    pub success: bool,
    pub message: String,
    /// 読んだ行数
    pub imported: usize,
    pub skipped: u32,
    pub summits: usize,
    pub rereview: usize,
}

impl From<SummitPotaImportResult> for PotaImportResultView {
    fn from(r: SummitPotaImportResult) -> Self {
        Self {
            success: true,
            message: format!(
                "{} 行({} 山頂)を取り込みました。要再確認になった山頂: {}",
                r.rows, r.summits, r.rereview
            ),
            imported: r.rows,
            skipped: 0,
            summits: r.summits,
            rereview: r.rereview,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct WhoAmIView {
    pub email: String,
    pub role: &'static str,
    pub method: AuthMethod,
}

impl From<&AdminIdentity> for WhoAmIView {
    fn from(i: &AdminIdentity) -> Self {
        Self {
            email: i.email.clone(),
            role: i.role.as_str(),
            method: i.method,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserView {
    pub email: String,
    pub role: &'static str,
    pub created_at: String,
}

impl From<AdminUser> for AdminUserView {
    fn from(u: AdminUser) -> Self {
        Self {
            email: u.email,
            role: u.role.as_str(),
            created_at: iso(u.created_at),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PutAdminUserRequest {
    pub role: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::model::sota::{
        SummitEstimateMeta, SummitPairDetail, SummitPotaEstimate, SummitPotaVerdict,
        SummitReviewDetail,
    };

    fn detail() -> ReviewedDetail {
        let est = |key: &str, code: &str, c| SummitPairDetail {
            estimate: SummitPotaEstimate {
                summit_code: "JA/KN-017".into(),
                muni_key: key.into(),
                muni_name: "市".into(),
                pota_code: code.into(),
                coverage: c,
                pct: Some(12.5),
                px: None,
                tot_px: None,
                dist_m: None,
                noise: false,
                shallow: false,
                hint: None,
                needs_review: false,
                note: None,
            },
            park_name: Some("公園".into()),
            park_inactive: false,
        };
        let d = SummitReviewDetail {
            summit_code: "JA/KN-017".into(),
            summit_name: "山".into(),
            alt_m: 1000,
            latitude: Some(35.0),
            longitude: None,
            city_j: Some("A市<JCC#1>B町<JCG#2B>".into()),
            activation_count: 3,
            meta: Some(SummitEstimateMeta {
                summit_code: "JA/KN-017".into(),
                input_rev: "rev1".into(),
                latitude: None,
                longitude: None,
                versions: DataVersions {
                    park: "p".into(),
                    muni: "m".into(),
                    munitable: "t".into(),
                },
                reasons: vec!["muni".into()],
                change_note: None,
                computed_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            }),
            pairs: vec![
                est("JCC#1", "JP-1", ParkCoverage::Part),
                est("JCC#9", "JP-2", ParkCoverage::None),
            ],
            verdicts: vec![
                SummitPotaVerdict {
                    muni_key: "JCC#1".into(),
                    pota_code: "JP-1".into(),
                    coverage: ParkCoverage::All,
                },
                SummitPotaVerdict {
                    muni_key: "JCC#OLD".into(),
                    pota_code: "JP-OLD".into(),
                    coverage: ParkCoverage::Part,
                },
            ],
            review: None,
            base_geojson: Some(r#"{"type":"FeatureCollection","features":[]}"#.into()),
            base_rev: Some("rev1".into()),
            served_at: None,
            log: vec![],
        };
        ReviewedDetail::new(d)
    }

    #[test]
    fn detail_view_json_shape() {
        let v = serde_json::to_value(SummitReviewDetailView::from(detail())).unwrap();
        assert_eq!(v["summitCode"], "JA/KN-017");
        assert_eq!(v["inputRev"], "rev1");
        assert_eq!(v["status"], "needs_check");
        assert_eq!(v["versions"]["munitable"], "t");
        // city_j に <muniKey> があるかどうか
        assert_eq!(v["pairs"][0]["inCityJ"], true);
        assert_eq!(v["pairs"][1]["inCityJ"], false);
        assert_eq!(v["pairs"][0]["estimate"], "part");
        assert_eq!(v["pairs"][0]["verdict"], "all");
        assert!(v["pairs"][1].get("verdict").is_none());
        // 今の候補にない組の目視は previousVerdicts に出る
        assert_eq!(v["previousVerdicts"][0]["muniKey"], "JCC#OLD");
        assert_eq!(v["previousVerdicts"].as_array().unwrap().len(), 1);
        // 下地は文字列ではなく JSON
        assert_eq!(v["baseMap"]["type"], "FeatureCollection");
        assert!(v.get("review").is_none());
        assert!(v.get("longitude").is_none());
    }

    #[test]
    fn confirm_request_parses_camel_case() {
        let r: ConfirmRequest = serde_json::from_str(
            r#"{"inputRev":"r","answers":[{"muniKey":"K","potaCode":"P","coverage":"part"}],"note":"メモ"}"#,
        )
        .unwrap();
        assert_eq!(r.input_rev, "r");
        assert_eq!(r.verdicts()[0].coverage, ParkCoverage::Part);
        assert_eq!(r.note.as_deref(), Some("メモ"));
        assert!(serde_json::from_str::<ConfirmRequest>(
            r#"{"inputRev":"r","answers":[{"muniKey":"K","potaCode":"P","coverage":"maybe"}]}"#
        )
        .is_err());
    }

    #[test]
    fn import_result_message() {
        let v: PotaImportResultView = SummitPotaImportResult {
            rows: 10,
            summits: 4,
            rereview: 2,
        }
        .into();
        assert_eq!(
            v.message,
            "10 行(4 山頂)を取り込みました。要再確認になった山頂: 2"
        );
        assert_eq!(v.imported, 10);
    }
}
