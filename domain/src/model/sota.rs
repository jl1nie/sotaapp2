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
}

impl ParkCoverage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Part => "part",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "part" => Some(Self::Part),
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
