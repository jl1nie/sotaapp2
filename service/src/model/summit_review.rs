//! 山頂の地図と POTA の判定の、DB に触れない純粋な関数。
//! 設計: docs/summit-map-admin-design.md 5・7・8 章

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use common::error::{AppError, AppResult};
use domain::model::sota::{
    ComposedPair, DataVersions, ParkCoverage, SummitEstimateMeta, SummitPairDetail,
    SummitPotaEstimate, SummitPotaVerdict, SummitReviewDetail, SummitReviewSummary,
};
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

// ---- 状態(5 章) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    NoMap,
    Unreviewed,
    NeedsCheck,
    Rereview,
    Confirmed,
}

/// stale の基準: 推定に使った位置と sota_references の位置が、これ以上離れている(m)
const STALE_DISTANCE_M: f64 = 20.0;

pub fn review_status(s: &SummitReviewSummary) -> ReviewStatus {
    if !s.has_map {
        return ReviewStatus::NoMap;
    }
    let current = s.meta.as_ref().map(|m| m.input_rev.as_str()).unwrap_or("");
    if let Some(r) = &s.review {
        return if r.input_rev == current {
            ReviewStatus::Confirmed
        } else {
            ReviewStatus::Rereview
        };
    }
    let has_reasons = s.meta.as_ref().is_some_and(|m| !m.reasons.is_empty());
    if s.part_pairs > 0 || s.flagged_pairs > 0 || has_reasons {
        ReviewStatus::NeedsCheck
    } else {
        ReviewStatus::Unreviewed
    }
}

/// 状態の計算に必要な入力(一覧と詳細で共通)
pub struct StatusInput<'a> {
    pub meta: Option<&'a SummitEstimateMeta>,
    pub ref_latitude: Option<f64>,
    pub ref_longitude: Option<f64>,
    pub base_rev: Option<&'a str>,
}

/// 推定が古い: 推定に使った位置と sota_references の位置が 20 m 以上離れている
pub fn is_stale(i: &StatusInput) -> bool {
    let (Some(m), Some(lat), Some(lon)) = (i.meta, i.ref_latitude, i.ref_longitude) else {
        return false;
    };
    let (Some(mlat), Some(mlon)) = (m.latitude, m.longitude) else {
        return false;
    };
    // calculate_distance はメートルで返す
    common::utils::calculate_distance(mlat, mlon, lat, lon) >= STALE_DISTANCE_M
}

/// 地図と推定の版がずれている: 下地を入れたときの input_rev が今と違う('legacy' は除く)
pub fn is_map_mismatch(i: &StatusInput) -> bool {
    let current = i.meta.map(|m| m.input_rev.as_str()).unwrap_or("");
    matches!(i.base_rev, Some(b) if b != "legacy" && b != current)
}

// ---- 一覧・詳細(9.1) ----

/// 一覧の絞り込み(`status` の値)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    Status(ReviewStatus),
    Stale,
    MapMismatch,
}

impl StatusFilter {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "no_map" => Self::Status(ReviewStatus::NoMap),
            "unreviewed" => Self::Status(ReviewStatus::Unreviewed),
            "needs_check" => Self::Status(ReviewStatus::NeedsCheck),
            "rereview" => Self::Status(ReviewStatus::Rereview),
            "confirmed" => Self::Status(ReviewStatus::Confirmed),
            "stale" => Self::Stale,
            "map_mismatch" => Self::MapMismatch,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusCounts {
    pub no_map: usize,
    pub unreviewed: usize,
    pub needs_check: usize,
    pub rereview: usize,
    pub confirmed: usize,
    pub stale: usize,
    pub map_mismatch: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummitReviewRow {
    pub summary: SummitReviewSummary,
    pub status: ReviewStatus,
    pub stale: bool,
    pub map_mismatch: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummitReviewList {
    /// prefix の範囲全体の件数(status・q で絞る前)
    pub counts: StatusCounts,
    pub rows: Vec<SummitReviewRow>,
}

impl SummitReviewRow {
    pub fn new(summary: SummitReviewSummary) -> Self {
        let input = StatusInput {
            meta: summary.meta.as_ref(),
            ref_latitude: summary.ref_latitude,
            ref_longitude: summary.ref_longitude,
            base_rev: summary.base_rev.as_deref(),
        };
        let (stale, map_mismatch) = (is_stale(&input), is_map_mismatch(&input));
        Self {
            status: review_status(&summary),
            stale,
            map_mismatch,
            summary,
        }
    }
}

/// 山頂コード・山名の部分一致(大文字小文字を区別しない)
fn matches_query(s: &SummitReviewSummary, q: &str) -> bool {
    let q = q.to_lowercase();
    s.summit_code.to_lowercase().contains(&q) || s.summit_name.to_lowercase().contains(&q)
}

/// 件数は絞る前の全体で数え、行は status・q で絞る
pub fn summarize(
    list: Vec<SummitReviewSummary>,
    filter: Option<StatusFilter>,
    q: Option<&str>,
) -> SummitReviewList {
    let mut counts = StatusCounts::default();
    let mut rows = Vec::new();
    let q = q.map(str::trim).filter(|q| !q.is_empty());
    for summary in list {
        let row = SummitReviewRow::new(summary);
        match row.status {
            ReviewStatus::NoMap => counts.no_map += 1,
            ReviewStatus::Unreviewed => counts.unreviewed += 1,
            ReviewStatus::NeedsCheck => counts.needs_check += 1,
            ReviewStatus::Rereview => counts.rereview += 1,
            ReviewStatus::Confirmed => counts.confirmed += 1,
        }
        counts.stale += row.stale as usize;
        counts.map_mismatch += row.map_mismatch as usize;

        let pass_status = match filter {
            None => true,
            Some(StatusFilter::Status(s)) => row.status == s,
            Some(StatusFilter::Stale) => row.stale,
            Some(StatusFilter::MapMismatch) => row.map_mismatch,
        };
        if pass_status && q.is_none_or(|q| matches_query(&row.summary, q)) {
            rows.push(row);
        }
    }
    SummitReviewList { counts, rows }
}

/// 詳細と、その状態
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewedDetail {
    pub detail: SummitReviewDetail,
    pub status: ReviewStatus,
    pub stale: bool,
    pub map_mismatch: bool,
}

impl ReviewedDetail {
    pub fn new(detail: SummitReviewDetail) -> Self {
        let row = SummitReviewRow::new(summary_of(&detail));
        Self {
            status: row.status,
            stale: row.stale,
            map_mismatch: row.map_mismatch,
            detail,
        }
    }
}

/// 詳細から、一覧の 1 行と同じ形を作る(状態の計算を共通にするため)
fn summary_of(d: &SummitReviewDetail) -> SummitReviewSummary {
    let count =
        |c: ParkCoverage| d.pairs.iter().filter(|p| p.estimate.coverage == c).count() as i64;
    SummitReviewSummary {
        summit_code: d.summit_code.clone(),
        summit_name: d.summit_name.clone(),
        ref_latitude: d.latitude,
        ref_longitude: d.longitude,
        meta: d.meta.clone(),
        has_map: d.base_geojson.is_some() || d.served_at.is_some(),
        base_rev: d.base_rev.clone(),
        pairs: d.pairs.len() as i64,
        all_pairs: count(ParkCoverage::All),
        part_pairs: count(ParkCoverage::Part),
        none_pairs: count(ParkCoverage::None),
        flagged_pairs: d
            .pairs
            .iter()
            .filter(|p| {
                let e = &p.estimate;
                e.needs_review
                    || e.noise
                    || e.shallow
                    || e.note.as_deref().is_some_and(|n| !n.is_empty())
            })
            .count() as i64,
        review: d.review.clone(),
    }
}

/// 目視の確定の入力
#[derive(Debug, Clone)]
pub struct ConfirmSummit {
    pub summit_code: String,
    pub input_rev: String,
    pub answers: Vec<SummitPotaVerdict>,
    pub note: Option<String>,
    pub reviewed_by: String,
}

/// 答えが今の推定の組とぴったり一致するか。足りない・余計・重複はどれも BadRequest
pub fn validate_answers(
    pairs: &[SummitPairDetail],
    answers: &[SummitPotaVerdict],
) -> AppResult<()> {
    let want: HashSet<(&str, &str)> = pairs
        .iter()
        .map(|p| (p.estimate.muni_key.as_str(), p.estimate.pota_code.as_str()))
        .collect();
    let got: HashSet<(&str, &str)> = answers
        .iter()
        .map(|a| (a.muni_key.as_str(), a.pota_code.as_str()))
        .collect();
    if got.len() != answers.len() {
        return Err(AppError::BadRequest(
            "答えに同じ組が重なっています".to_string(),
        ));
    }
    if want != got {
        return Err(AppError::BadRequest(
            "答えの組が、今の推定の組と一致しません".to_string(),
        ));
    }
    Ok(())
}

/// 配信用の地図に差し込む答え。無効・未登録の公園の組は入れない(配信の規則と同じ)
pub fn composed_for_map(
    pairs: &[SummitPairDetail],
    coverage: impl Fn(&SummitPairDetail) -> ParkCoverage,
) -> Vec<ComposedPair> {
    pairs
        .iter()
        .filter(|p| !p.park_inactive)
        .map(|p| ComposedPair {
            muni_key: p.estimate.muni_key.clone(),
            pota_code: p.estimate.pota_code.clone(),
            park_name: p.park_name.clone(),
            coverage: coverage(p),
        })
        .collect()
}

// ---- 地図の差し込み(7 章) ----

/// 下地の GeoJSON に、組ごとの答えを差し込んで配信用の地図を作る。座標には触らない
pub fn build_served_map(base: &str, pairs: &[ComposedPair]) -> AppResult<String> {
    let mut root: Value = serde_json::from_str(base)
        .map_err(|e| AppError::UnprocessableEntity(format!("地図の JSON を読めません: {e}")))?;
    let features = root
        .get_mut("features")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| AppError::UnprocessableEntity("features がありません".to_string()))?;

    let visible = |c: ParkCoverage| matches!(c, ParkCoverage::All | ParkCoverage::Part);
    let coverage_of: HashMap<(&str, &str), ParkCoverage> = pairs
        .iter()
        .map(|p| ((p.muni_key.as_str(), p.pota_code.as_str()), p.coverage))
        .collect();

    features.retain_mut(|f| {
        let kind = f
            .pointer("/properties/kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        match kind.as_str() {
            // 判定用の層は端末には送らない
            "judge_park" | "peak" => false,
            "muni" => {
                let key = f
                    .pointer("/properties/key")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let mut list: Vec<&ComposedPair> = pairs
                    .iter()
                    .filter(|p| p.muni_key == key && visible(p.coverage))
                    .collect();
                list.sort_by(|a, b| a.pota_code.cmp(&b.pota_code));
                let pota: Vec<Value> = list
                    .into_iter()
                    .map(|p| {
                        let mut o = Map::new();
                        o.insert("code".into(), json!(p.pota_code));
                        if let Some(n) = &p.park_name {
                            o.insert("name".into(), json!(n));
                        }
                        o.insert("coverage".into(), json!(p.coverage.as_str()));
                        Value::Object(o)
                    })
                    .collect();
                if let Some(props) = f.get_mut("properties").and_then(Value::as_object_mut) {
                    props.insert("pota".into(), Value::Array(pota));
                }
                true
            }
            // 公園の区域は、答えが「一部」の組だけ残す
            "park" => {
                let code = f.pointer("/properties/code").and_then(Value::as_str);
                let key = f.pointer("/properties/key").and_then(Value::as_str);
                matches!((code, key), (Some(c), Some(k))
                    if coverage_of.get(&(k, c)) == Some(&ParkCoverage::Part))
            }
            _ => true,
        }
    });
    serde_json::to_string(&root)
        .map_err(|e| AppError::UnprocessableEntity(format!("地図の JSON を作れません: {e}")))
}

// ---- 推定の CSV(8 章) ----

/// 読んだ結果。`estimates` と `metas` は同じ山頂の集合を指す
#[derive(Debug)]
pub struct ParsedEstimates {
    pub rows: usize,
    pub estimates: Vec<SummitPotaEstimate>,
    pub metas: Vec<SummitEstimateMeta>,
}

fn unprocessable(msg: impl Into<String>) -> AppError {
    AppError::UnprocessableEntity(msg.into())
}

/// 空の欄は None
fn opt(rec: &csv::StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|i| rec.get(i))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn parse_num<T: std::str::FromStr>(
    line: usize,
    name: &str,
    v: Option<String>,
) -> AppResult<Option<T>> {
    v.map(|s| {
        s.parse::<T>()
            .map_err(|_| unprocessable(format!("{line} 行目: {name} が数ではありません: {s}")))
    })
    .transpose()
}

fn parse_flag(line: usize, name: &str, v: Option<String>) -> AppResult<bool> {
    match v.as_deref() {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(s) => Err(unprocessable(format!(
            "{line} 行目: {name} は 0 か 1 です: {s}"
        ))),
    }
}

fn parse_coverage(line: usize, name: &str, s: &str) -> AppResult<ParkCoverage> {
    ParkCoverage::parse(s)
        .ok_or_else(|| unprocessable(format!("{line} 行目: {name} は all・part・none です: {s}")))
}

/// 山頂の列(同じ山頂の行で同じ値)
#[derive(Default, PartialEq, Clone)]
struct SummitCols {
    input_rev: Option<String>,
    latitude: Option<String>,
    longitude: Option<String>,
    park_version: Option<String>,
    muni_version: Option<String>,
    munitable_version: Option<String>,
    reasons: Option<String>,
    change_note: Option<String>,
}

/// 見出しの名前で読む。`now` は meta の computed_at に入れる
pub fn parse_estimates_csv(data: &str, now: DateTime<Utc>) -> AppResult<ParsedEstimates> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(data.as_bytes());
    let headers = rdr.headers().map_err(AppError::CSVReadError)?.clone();
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let need =
        |name: &str| col(name).ok_or_else(|| unprocessable(format!("見出し {name} がありません")));
    let (c_summit, c_key, c_muni, c_pota, c_cov) = (
        need("summit_code")?,
        need("muni_key")?,
        need("muni_name")?,
        need("pota_code")?,
        need("coverage")?,
    );
    let (c_pct, c_px, c_tot, c_dist) = (col("pct"), col("px"), col("tot_px"), col("dist_m"));
    let (c_noise, c_shallow, c_hint, c_needs, c_note) = (
        col("noise"),
        col("shallow"),
        col("hint"),
        col("needs_review"),
        col("note"),
    );
    let sc = (
        col("input_rev"),
        col("latitude"),
        col("longitude"),
        col("park_version"),
        col("muni_version"),
        col("munitable_version"),
        col("reasons"),
        col("change_note"),
    );

    let mut estimates = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut cols: HashMap<String, SummitCols> = HashMap::new();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut rows = 0usize;

    for (i, result) in rdr.records().enumerate() {
        let line = i + 2;
        let rec = result.map_err(AppError::CSVReadError)?;
        let get = |c: usize| rec.get(c).map(str::trim).unwrap_or("");
        let summit_code = get(c_summit).to_string();
        let muni_key = get(c_key).to_string();
        let pota_code = get(c_pota).to_string();
        if summit_code.is_empty() || muni_key.is_empty() || pota_code.is_empty() {
            return Err(unprocessable(format!(
                "{line} 行目: summit_code・muni_key・pota_code は必須です"
            )));
        }
        if !seen.insert((summit_code.clone(), muni_key.clone(), pota_code.clone())) {
            return Err(unprocessable(format!(
                "{line} 行目: 同じ組が 2 回あります: {summit_code} {muni_key} {pota_code}"
            )));
        }
        let this = SummitCols {
            input_rev: opt(&rec, sc.0),
            latitude: opt(&rec, sc.1),
            longitude: opt(&rec, sc.2),
            park_version: opt(&rec, sc.3),
            muni_version: opt(&rec, sc.4),
            munitable_version: opt(&rec, sc.5),
            reasons: opt(&rec, sc.6),
            change_note: opt(&rec, sc.7),
        };
        match cols.get(&summit_code) {
            Some(prev) if *prev != this => {
                return Err(unprocessable(format!(
                    "{line} 行目: {summit_code} の山頂の列が、ほかの行と食い違っています"
                )));
            }
            Some(_) => {}
            None => {
                order.push(summit_code.clone());
                cols.insert(summit_code.clone(), this);
            }
        }

        estimates.push(SummitPotaEstimate {
            summit_code,
            muni_key,
            muni_name: get(c_muni).to_string(),
            pota_code,
            coverage: parse_coverage(line, "coverage", get(c_cov))?,
            pct: parse_num(line, "pct", opt(&rec, c_pct))?,
            px: parse_num(line, "px", opt(&rec, c_px))?,
            tot_px: parse_num(line, "tot_px", opt(&rec, c_tot))?,
            dist_m: parse_num(line, "dist_m", opt(&rec, c_dist))?,
            noise: parse_flag(line, "noise", opt(&rec, c_noise))?,
            shallow: parse_flag(line, "shallow", opt(&rec, c_shallow))?,
            hint: opt(&rec, c_hint)
                .map(|h| parse_coverage(line, "hint", &h))
                .transpose()?,
            needs_review: parse_flag(line, "needs_review", opt(&rec, c_needs))?,
            note: opt(&rec, c_note),
        });
        rows += 1;
    }

    let mut metas = Vec::with_capacity(order.len());
    for code in order {
        let c = cols.remove(&code).unwrap_or_default();
        let input_rev = c.input_rev.unwrap_or_else(|| {
            pairs_fingerprint(
                estimates
                    .iter()
                    .filter(|e| e.summit_code == code)
                    .map(|e| (e.muni_key.as_str(), e.pota_code.as_str())),
            )
        });
        let unknown = || "unknown".to_string();
        metas.push(SummitEstimateMeta {
            input_rev,
            latitude: parse_num(0, "latitude", c.latitude)?,
            longitude: parse_num(0, "longitude", c.longitude)?,
            versions: DataVersions {
                park: c.park_version.unwrap_or_else(unknown),
                muni: c.muni_version.unwrap_or_else(unknown),
                munitable: c.munitable_version.unwrap_or_else(unknown),
            },
            reasons: c
                .reasons
                .map(|r| {
                    r.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            change_note: c.change_note,
            computed_at: now,
            summit_code: code,
        });
    }

    Ok(ParsedEstimates {
        rows,
        estimates,
        metas,
    })
}

/// input_rev がない山頂の指紋: 組を並べて改行でつないだものの SHA-256 の先頭 16 桁に `pairs:` を付ける
fn pairs_fingerprint<'a>(pairs: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    let mut lines: Vec<String> = pairs.map(|(k, p)| format!("{k}|{p}")).collect();
    lines.sort();
    let digest = Sha256::digest(lines.join("\n").as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("pairs:{}", &hex[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::model::sota::SummitReview;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn versions() -> DataVersions {
        DataVersions {
            park: "p".into(),
            muni: "m".into(),
            munitable: "t".into(),
        }
    }

    fn meta(rev: &str, reasons: &[&str], lat: f64, lon: f64) -> SummitEstimateMeta {
        SummitEstimateMeta {
            summit_code: "JA/KN-017".into(),
            input_rev: rev.into(),
            latitude: Some(lat),
            longitude: Some(lon),
            versions: versions(),
            reasons: reasons.iter().map(|s| s.to_string()).collect(),
            change_note: None,
            computed_at: now(),
        }
    }

    fn summary(
        has_map: bool,
        meta: Option<SummitEstimateMeta>,
        review_rev: Option<&str>,
        part: i64,
        flagged: i64,
    ) -> SummitReviewSummary {
        SummitReviewSummary {
            summit_code: "JA/KN-017".into(),
            summit_name: "x".into(),
            ref_latitude: None,
            ref_longitude: None,
            meta,
            has_map,
            base_rev: None,
            pairs: 1,
            all_pairs: 0,
            part_pairs: part,
            none_pairs: 0,
            flagged_pairs: flagged,
            review: review_rev.map(|r| SummitReview {
                summit_code: "JA/KN-017".into(),
                input_rev: r.into(),
                reviewed_by: "a@example.com".into(),
                versions: versions(),
                reviewed_at: now(),
                note: None,
            }),
        }
    }

    #[test]
    fn status_rules() {
        let m = || Some(meta("r1", &[], 35.0, 139.0));
        assert_eq!(
            review_status(&summary(false, m(), Some("r1"), 0, 0)),
            ReviewStatus::NoMap
        );
        assert_eq!(
            review_status(&summary(true, m(), Some("r1"), 1, 1)),
            ReviewStatus::Confirmed
        );
        assert_eq!(
            review_status(&summary(true, m(), Some("r0"), 0, 0)),
            ReviewStatus::Rereview
        );
        assert_eq!(
            review_status(&summary(true, m(), None, 1, 0)),
            ReviewStatus::NeedsCheck
        );
        assert_eq!(
            review_status(&summary(true, m(), None, 0, 1)),
            ReviewStatus::NeedsCheck
        );
        assert_eq!(
            review_status(&summary(true, m(), None, 0, 0)),
            ReviewStatus::Unreviewed
        );
        let with_reasons = Some(meta("r1", &["muni"], 35.0, 139.0));
        assert_eq!(
            review_status(&summary(true, with_reasons, None, 0, 0)),
            ReviewStatus::NeedsCheck
        );
        // meta がない山頂の確定は、input_rev が "" と一致しないので要再確認
        assert_eq!(
            review_status(&summary(true, None, Some("r1"), 0, 0)),
            ReviewStatus::Rereview
        );
    }

    #[test]
    fn stale_and_map_mismatch() {
        let m = meta("r1", &[], 35.0, 139.0);
        let near = StatusInput {
            meta: Some(&m),
            ref_latitude: Some(35.00005),
            ref_longitude: Some(139.0),
            base_rev: None,
        };
        let far = StatusInput {
            meta: Some(&m),
            ref_latitude: Some(35.001),
            ref_longitude: Some(139.0),
            base_rev: None,
        };
        let none = StatusInput {
            meta: Some(&m),
            ref_latitude: None,
            ref_longitude: Some(139.0),
            base_rev: None,
        };
        assert!(!is_stale(&near));
        assert!(is_stale(&far));
        assert!(!is_stale(&none));

        let with = |b| StatusInput {
            meta: Some(&m),
            ref_latitude: None,
            ref_longitude: None,
            base_rev: b,
        };
        assert!(!is_map_mismatch(&with(None)));
        assert!(!is_map_mismatch(&with(Some("legacy"))));
        assert!(!is_map_mismatch(&with(Some("r1"))));
        assert!(is_map_mismatch(&with(Some("r0"))));
    }

    fn pair(key: &str, code: &str, name: Option<&str>, c: ParkCoverage) -> ComposedPair {
        ComposedPair {
            muni_key: key.into(),
            pota_code: code.into(),
            park_name: name.map(str::to_string),
            coverage: c,
        }
    }

    const BASE: &str = r#"{"type":"FeatureCollection","summit":"JA/KN-017","bbox":[1,2,3,4],"features":[
        {"type":"Feature","properties":{"kind":"zone"},"geometry":null},
        {"type":"Feature","properties":{"kind":"muni","key":"JCC#1","name":"A市","pota":[{"code":"OLD","coverage":"all"}]},"geometry":null},
        {"type":"Feature","properties":{"kind":"park","code":"JP-1","key":"JCC#1"},"geometry":null},
        {"type":"Feature","properties":{"kind":"park","code":"JP-2","key":"JCC#1"},"geometry":null},
        {"type":"Feature","properties":{"kind":"judge_park","code":"JP-1","key":"JCC#1"},"geometry":null},
        {"type":"Feature","properties":{"kind":"peak"},"geometry":null},
        {"type":"Feature","properties":{"kind":"summit","code":"JA/KN-017"},"geometry":null}]}"#;

    #[test]
    fn served_map_rewrites_pota_and_filters_layers() {
        let pairs = vec![
            pair("JCC#1", "JP-2", Some("二"), ParkCoverage::All),
            pair("JCC#1", "JP-1", None, ParkCoverage::Part),
            pair("JCC#1", "JP-3", Some("三"), ParkCoverage::None),
        ];
        let out: Value = serde_json::from_str(&build_served_map(BASE, &pairs).unwrap()).unwrap();
        assert_eq!(out["summit"], "JA/KN-017");
        assert_eq!(out["bbox"], json!([1, 2, 3, 4]));
        let kinds: Vec<&str> = out["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["properties"]["kind"].as_str().unwrap())
            .collect();
        // judge_park・peak は外れ、park は「一部」の JP-1 だけ残る
        assert_eq!(kinds, vec!["zone", "muni", "park", "summit"]);
        let muni = &out["features"][1]["properties"];
        // code の順。名前がなければ name を入れない。none は入れない
        assert_eq!(
            muni["pota"],
            json!([{"code":"JP-1","coverage":"part"},{"code":"JP-2","name":"二","coverage":"all"}])
        );
        assert_eq!(out["features"][2]["properties"]["code"], "JP-1");
        // つめた 1 行
        assert!(!build_served_map(BASE, &pairs).unwrap().contains('\n'));
    }

    #[test]
    fn served_map_with_no_answers_clears_pota() {
        let out: Value = serde_json::from_str(&build_served_map(BASE, &[]).unwrap()).unwrap();
        assert_eq!(out["features"][1]["properties"]["pota"], json!([]));
        assert_eq!(out["features"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn served_map_rejects_broken_base() {
        assert!(build_served_map("not json", &[]).is_err());
        assert!(build_served_map("{}", &[]).is_err());
    }

    #[test]
    fn csv_legacy_six_columns() {
        let csv = "summit_code,muni_key,muni_name,pota_code,coverage,verified\n\
                   JA/KN-017,JCC#1,A市,JP-1,part,0\n\
                   JA/KN-017,JCC#1,A市,JP-2,all,1\n\
                   JA/KN-018,JCC#2,B市,JP-1,none,0\n";
        let p = parse_estimates_csv(csv, now()).unwrap();
        assert_eq!(p.rows, 3);
        assert_eq!(p.metas.len(), 2);
        assert_eq!(p.metas[0].summit_code, "JA/KN-017");
        assert!(p.metas[0].input_rev.starts_with("pairs:"));
        assert_eq!(p.metas[0].input_rev.len(), "pairs:".len() + 16);
        assert_eq!(p.metas[0].versions.park, "unknown");
        assert_eq!(p.metas[0].latitude, None);
        assert_eq!(p.estimates[2].coverage, ParkCoverage::None);
    }

    #[test]
    fn csv_fingerprint_follows_pairs_not_order() {
        let a = "summit_code,muni_key,muni_name,pota_code,coverage\nS,K,n,P1,all\nS,K,n,P2,part\n";
        let b = "summit_code,muni_key,muni_name,pota_code,coverage\nS,K,n,P2,part\nS,K,n,P1,all\n";
        let c = "summit_code,muni_key,muni_name,pota_code,coverage\nS,K,n,P1,all\n";
        let rev = |s| {
            parse_estimates_csv(s, now()).unwrap().metas[0]
                .input_rev
                .clone()
        };
        assert_eq!(rev(a), rev(b));
        assert_ne!(rev(a), rev(c));
    }

    #[test]
    fn csv_new_columns() {
        let csv = "summit_code,muni_key,muni_name,pota_code,coverage,pct,px,tot_px,dist_m,noise,shallow,hint,needs_review,note,input_rev,latitude,longitude,park_version,muni_version,munitable_version,reasons,change_note\n\
                   S,K,n,P1,part,12.5,30,240,,1,0,all,1,メモ,rev9,35.5,139.5,v24,N03,t1,\"muni,near\",差分\n\
                   S,K,n,P2,none,,,,,,,,,,rev9,35.5,139.5,v24,N03,t1,\"muni,near\",差分\n";
        let p = parse_estimates_csv(csv, now()).unwrap();
        let e = &p.estimates[0];
        assert_eq!(
            (e.pct, e.px, e.tot_px, e.dist_m),
            (Some(12.5), Some(30), Some(240), None)
        );
        assert!(e.noise && !e.shallow && e.needs_review);
        assert_eq!(e.hint, Some(ParkCoverage::All));
        assert_eq!(e.note.as_deref(), Some("メモ"));
        let m = &p.metas[0];
        assert_eq!(m.input_rev, "rev9");
        assert_eq!((m.latitude, m.longitude), (Some(35.5), Some(139.5)));
        assert_eq!(m.versions.munitable, "t1");
        assert_eq!(m.reasons, vec!["muni", "near"]);
        assert_eq!(m.change_note.as_deref(), Some("差分"));
    }

    #[test]
    fn csv_rejects_bad_input() {
        let head = "summit_code,muni_key,muni_name,pota_code,coverage,input_rev\n";
        let dup = format!("{head}S,K,n,P1,all,r\nS,K,n,P1,part,r\n");
        let mismatch = format!("{head}S,K,n,P1,all,r1\nS,K,n,P2,all,r2\n");
        let bad_cov = format!("{head}S,K,n,P1,maybe,r\n");
        let no_col = "summit_code,muni_key\nS,K\n".to_string();
        for s in [dup, mismatch, bad_cov, no_col] {
            assert!(matches!(
                parse_estimates_csv(&s, now()),
                Err(AppError::UnprocessableEntity(_))
            ));
        }
    }

    fn row(code: &str, name: &str, has_map: bool, part: i64) -> SummitReviewSummary {
        let mut s = summary(has_map, Some(meta("r1", &[], 35.0, 139.0)), None, part, 0);
        s.summit_code = code.into();
        s.summit_name = name.into();
        s
    }

    #[test]
    fn summarize_counts_before_filtering() {
        let list = vec![
            row("JA/KN-001", "Fuji", true, 0),
            row("JA/KN-002", "Tanzawa", true, 1),
            row("JA/KN-003", "Hakone", false, 0),
        ];
        let r = summarize(
            list.clone(),
            Some(StatusFilter::Status(ReviewStatus::NeedsCheck)),
            None,
        );
        assert_eq!(r.counts.unreviewed, 1);
        assert_eq!(r.counts.needs_check, 1);
        assert_eq!(r.counts.no_map, 1);
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].summary.summit_code, "JA/KN-002");

        // q は山頂コード・山名の部分一致で、大文字小文字を区別しない
        let r = summarize(list.clone(), None, Some("HAKO"));
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.counts.no_map, 1);
        assert_eq!(summarize(list.clone(), None, Some("kn-00")).rows.len(), 3);
        assert_eq!(summarize(list, None, Some("  ")).rows.len(), 3);
    }

    #[test]
    fn status_filter_parse() {
        assert_eq!(StatusFilter::parse("stale"), Some(StatusFilter::Stale));
        assert_eq!(
            StatusFilter::parse("needs_check"),
            Some(StatusFilter::Status(ReviewStatus::NeedsCheck))
        );
        assert_eq!(StatusFilter::parse("x"), None);
    }

    fn pair_detail(key: &str, code: &str, inactive: bool) -> SummitPairDetail {
        SummitPairDetail {
            estimate: SummitPotaEstimate {
                summit_code: "S".into(),
                muni_key: key.into(),
                muni_name: "n".into(),
                pota_code: code.into(),
                coverage: ParkCoverage::Part,
                pct: None,
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
            park_inactive: inactive,
        }
    }

    fn verdict(key: &str, code: &str) -> SummitPotaVerdict {
        SummitPotaVerdict {
            muni_key: key.into(),
            pota_code: code.into(),
            coverage: ParkCoverage::All,
        }
    }

    #[test]
    fn validate_answers_requires_exact_set() {
        let pairs = vec![pair_detail("K", "P1", false), pair_detail("K", "P2", false)];
        assert!(validate_answers(&pairs, &[verdict("K", "P2"), verdict("K", "P1")]).is_ok());
        // 足りない
        assert!(matches!(
            validate_answers(&pairs, &[verdict("K", "P1")]),
            Err(AppError::BadRequest(_))
        ));
        // 余計
        assert!(matches!(
            validate_answers(
                &pairs,
                &[verdict("K", "P1"), verdict("K", "P2"), verdict("K", "P3")]
            ),
            Err(AppError::BadRequest(_))
        ));
        // 重複
        assert!(matches!(
            validate_answers(&pairs, &[verdict("K", "P1"), verdict("K", "P1")]),
            Err(AppError::BadRequest(_))
        ));
        // 候補が 0 の山頂は、空の答えで確定できる
        assert!(validate_answers(&[], &[]).is_ok());
    }

    #[test]
    fn composed_for_map_skips_inactive_parks() {
        let pairs = vec![pair_detail("K", "P1", false), pair_detail("K", "P2", true)];
        let c = composed_for_map(&pairs, |_| ParkCoverage::All);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].pota_code, "P1");
        assert_eq!(c[0].coverage, ParkCoverage::All);
    }
}
