use chrono::{DateTime, NaiveDate, Utc};

use super::{id::UserId, Maidenhead};

pub struct SummitCode(String);
impl SummitCode {
    pub fn new(code: String) -> Self {
        Self(code)
    }
    pub fn inner_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct SotaReference {
    pub summit_code: String,
    pub association_name: String,
    pub region_name: String,
    pub summit_name: String,
    pub summit_name_j: Option<String>,
    pub city: Option<String>,
    pub city_j: Option<String>,
    pub alt_m: i32,
    pub alt_ft: i32,
    pub grid_ref1: String,
    pub grid_ref2: String,
    pub longitude: f64,
    pub latitude: f64,
    pub maidenhead: Maidenhead,
    pub points: i32,
    pub bonus_points: i32,
    pub valid_from: NaiveDate,
    pub valid_to: NaiveDate,
    pub activation_count: i32,
    pub activation_date: Option<String>,
    pub activation_call: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SotaLog {
    pub user_id: UserId,
    pub my_callsign: String,
    pub operator: String,
    pub my_summit_code: Option<String>,
    pub time: DateTime<Utc>,
    pub frequency: String,
    pub mode: String,
    pub his_callsign: String,
    pub his_summit_code: Option<String>,
    pub comment: Option<String>,
    pub update: DateTime<Utc>,
}

/// 市区町村の中のゾーンのうち、公園に入る範囲
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkCoverage {
    /// 全部入る(その市区町村を選べば POTA になる)
    All,
    /// 一部だけ入る(現地で公園の中にいるかを確かめる)
    Part,
    /// 入らない(推定・目視の答えとしてだけ使う。配信には入らない)
    None,
}

impl ParkCoverage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Part => "part",
            Self::None => "none",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "part" => Some(Self::Part),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// 山頂で、市区町村(JCC/JCG)を選んだときに当たる POTA の公園
#[derive(Debug, Clone, PartialEq)]
pub struct SummitPota {
    pub summit_code: String,
    pub muni_key: String,
    pub muni_name: String,
    pub pota_code: String,
    pub park_name: Option<String>,
    pub coverage: ParkCoverage,
    /// 目視で確かめた答えなら true、推定なら false
    pub verified: bool,
}

/// 山頂の簡易地図(GeoJSON の FeatureCollection の文字列)
#[derive(Debug, Clone, PartialEq)]
pub struct SummitMap {
    pub summit_code: String,
    pub geojson: String,
}

/// 推定に使ったデータの版
#[derive(Debug, Clone, PartialEq)]
pub struct DataVersions {
    pub park: String,
    pub muni: String,
    pub munitable: String,
}

/// 推定の 1 組(山頂 × 市区町村 × 公園)。none も含む
#[derive(Debug, Clone, PartialEq)]
pub struct SummitPotaEstimate {
    pub summit_code: String,
    pub muni_key: String,
    pub muni_name: String,
    pub pota_code: String,
    pub coverage: ParkCoverage,
    pub pct: Option<f64>,
    pub px: Option<i64>,
    pub tot_px: Option<i64>,
    pub dist_m: Option<f64>,
    pub noise: bool,
    pub shallow: bool,
    pub hint: Option<ParkCoverage>,
    pub needs_review: bool,
    pub note: Option<String>,
}

/// 山頂ごとの、推定を計算したときの入力
#[derive(Debug, Clone, PartialEq)]
pub struct SummitEstimateMeta {
    pub summit_code: String,
    pub input_rev: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub versions: DataVersions,
    pub reasons: Vec<String>,
    pub change_note: Option<String>,
    pub computed_at: DateTime<Utc>,
}

/// 目視の答え
#[derive(Debug, Clone, PartialEq)]
pub struct SummitPotaVerdict {
    pub muni_key: String,
    pub pota_code: String,
    pub coverage: ParkCoverage,
}

/// 山頂ごとの確定の記録
#[derive(Debug, Clone, PartialEq)]
pub struct SummitReview {
    pub summit_code: String,
    pub input_rev: String,
    /// 確定した人のメール
    pub reviewed_by: String,
    pub versions: DataVersions,
    pub reviewed_at: DateTime<Utc>,
    pub note: Option<String>,
}

/// 答えの変更の履歴の 1 行
#[derive(Debug, Clone, PartialEq)]
pub struct SummitPotaLogEntry {
    pub muni_key: String,
    pub pota_code: String,
    pub before: Option<ParkCoverage>,
    pub after: Option<ParkCoverage>,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
}

/// 配信用の地図を作るための、組ごとの答え(目視があれば目視、なければ推定)
#[derive(Debug, Clone, PartialEq)]
pub struct ComposedPair {
    pub muni_key: String,
    pub pota_code: String,
    pub park_name: Option<String>,
    pub coverage: ParkCoverage,
}

/// 地図の作り直しの入力(下地と答え)
#[derive(Debug, Clone, PartialEq)]
pub struct SummitMapInput {
    pub summit_code: String,
    pub base_geojson: String,
    pub pairs: Vec<ComposedPair>,
}

/// 見直しの一覧の 1 行(状態は service で計算する)
#[derive(Debug, Clone, PartialEq)]
pub struct SummitReviewSummary {
    pub summit_code: String,
    pub summit_name: String,
    pub ref_latitude: Option<f64>,
    pub ref_longitude: Option<f64>,
    pub meta: Option<SummitEstimateMeta>,
    pub has_map: bool,
    pub base_rev: Option<String>,
    pub pairs: i64,
    pub all_pairs: i64,
    pub part_pairs: i64,
    pub none_pairs: i64,
    /// needs_review・noise・shallow のどれか、または note がある組
    pub flagged_pairs: i64,
    pub review: Option<SummitReview>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummitPairDetail {
    pub estimate: SummitPotaEstimate,
    pub park_name: Option<String>,
    /// park_inactive、または pota_references にない
    pub park_inactive: bool,
}

/// 見直しの詳細(管理画面の 1 山頂)
#[derive(Debug, Clone, PartialEq)]
pub struct SummitReviewDetail {
    pub summit_code: String,
    pub summit_name: String,
    pub alt_m: i32,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub city_j: Option<String>,
    pub activation_count: i64,
    pub meta: Option<SummitEstimateMeta>,
    pub pairs: Vec<SummitPairDetail>,
    /// 今の候補にない組の目視も含めて全部
    pub verdicts: Vec<SummitPotaVerdict>,
    pub review: Option<SummitReview>,
    pub base_geojson: Option<String>,
    pub base_rev: Option<String>,
    pub served_at: Option<DateTime<Utc>>,
    /// 新しい順、200 件まで
    pub log: Vec<SummitPotaLogEntry>,
}

/// 目視の確定
#[derive(Debug, Clone)]
pub struct SaveSummitReview {
    pub summit_code: String,
    pub expected_input_rev: String,
    pub answers: Vec<SummitPotaVerdict>,
    pub reviewed_by: String,
    pub note: Option<String>,
    pub now: DateTime<Utc>,
    pub served_geojson: Option<String>,
}

/// 目視の取り消し
#[derive(Debug, Clone)]
pub struct DeleteSummitReview {
    pub summit_code: String,
    pub changed_by: String,
    pub now: DateTime<Utc>,
    pub served_geojson: Option<String>,
}
