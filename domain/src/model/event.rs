use aprs_message::AprsCallsign;
use chrono::{DateTime, Utc};
use std::str::FromStr;

use derive_new::new;

use crate::model::{id::LogId, AwardProgram};
use crate::model::{pota::PotaRefLog, sota::SotaReference};

#[derive(new, Debug)]
pub struct BoundingBox {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

#[derive(new, Debug)]
pub struct CenterRadius {
    pub lon: f64,
    pub lat: f64,
    pub rad: f64,
}

#[derive(Default, Debug)]
pub struct FindRef {
    pub program: Vec<AwardProgram>,
    pub sota_code: Option<String>,
    pub pota_code: Option<String>,
    pub wwff_code: Option<String>,
    pub name: Option<String>,
    pub lon: Option<f64>,
    pub lat: Option<f64>,
    pub bbox: Option<BoundingBox>,
    pub center: Option<CenterRadius>,
    pub min_elev: Option<i32>,
    pub min_area: Option<i32>,
    pub log_id: Option<LogId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl FindRef {
    pub fn is_sota(&self) -> bool {
        self.program.contains(&AwardProgram::SOTA)
    }

    pub fn is_pota(&self) -> bool {
        self.program.contains(&AwardProgram::POTA)
    }

    pub fn is_wwff(&self) -> bool {
        self.program.contains(&AwardProgram::WWFF)
    }
}

pub struct FindRefBuilder {
    pub param: FindRef,
}

impl Default for FindRefBuilder {
    fn default() -> Self {
        let param = FindRef {
            program: Vec::<AwardProgram>::new(),
            ..Default::default()
        };
        Self { param }
    }
}

impl FindRefBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn program(mut self, program: AwardProgram) -> Self {
        self.param.program.push(program);
        self
    }

    pub fn sota(mut self) -> Self {
        self.param.program.push(AwardProgram::SOTA);
        self
    }

    pub fn pota(mut self) -> Self {
        self.param.program.push(AwardProgram::POTA);
        self
    }

    pub fn wwff(mut self) -> Self {
        self.param.program.push(AwardProgram::WWFF);
        self
    }

    pub fn sota_code(mut self, code: String) -> Self {
        self.param.sota_code = Some(code.to_uppercase());
        self
    }

    pub fn pota_code(mut self, code: String) -> Self {
        self.param.pota_code = Some(code.to_uppercase());
        self
    }

    pub fn wwff_code(mut self, code: String) -> Self {
        self.param.wwff_code = Some(code.to_uppercase());
        self
    }

    pub fn name(mut self, n: String) -> Self {
        self.param.name = Some(n);
        self
    }

    pub fn lon(mut self, lon: f64) -> Self {
        self.param.lon = Some(lon);
        self
    }

    pub fn lat(mut self, lat: f64) -> Self {
        self.param.lat = Some(lat);
        self
    }

    pub fn bbox(mut self, min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Self {
        let bbox = BoundingBox::new(min_lon, min_lat, max_lon, max_lat);
        self.param.bbox = Some(bbox);
        self
    }

    pub fn center(mut self, lon: f64, lat: f64, radius: f64) -> Self {
        let cr = CenterRadius::new(lon, lat, radius);
        self.param.center = Some(cr);
        self
    }

    pub fn min_elev(mut self, elev: i32) -> Self {
        self.param.min_elev = Some(elev);
        self
    }

    pub fn min_area(mut self, area: i32) -> Self {
        self.param.min_area = Some(area);
        self
    }

    pub fn log_id(mut self, id: LogId) -> Self {
        self.param.log_id = Some(id);
        self
    }

    pub fn limit(mut self, l: i32) -> Self {
        self.param.limit = Some(l);
        self
    }

    pub fn offset(mut self, o: i32) -> Self {
        self.param.offset = Some(o);
        self
    }

    pub fn build(self) -> FindRef {
        self.param
    }
}

#[derive(Default)]
pub struct FindResult {
    pub sota: Option<Vec<SotaReference>>,
    pub pota: Option<Vec<PotaRefLog>>,
}

#[derive(Default, Debug)]
pub struct PagenatedResult<T> {
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
    pub results: Vec<T>,
}

#[derive(Debug)]
pub enum DeleteRef<T> {
    Delete(T),
    DeleteAll,
}

#[derive(Debug, Default)]
pub struct DeleteLog {
    pub before: Option<DateTime<Utc>>,
    pub log_id: Option<LogId>,
}

#[derive(Debug, Default)]
pub struct FindLog {
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub activation: bool,
}

#[derive(Default)]
pub struct FindLogBuilder {
    param: FindLog,
}

impl FindLogBuilder {
    pub fn after(mut self, after: DateTime<Utc>) -> Self {
        self.param.after = Some(after);
        self
    }

    pub fn before(mut self, before: DateTime<Utc>) -> Self {
        self.param.before = Some(before);
        self
    }

    pub fn activation(mut self) -> Self {
        self.param.activation = true;
        self
    }

    pub fn chase(mut self) -> Self {
        self.param.activation = false;
        self
    }

    pub fn build(self) -> FindLog {
        self.param
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GroupBy {
    Callsign(Option<String>),
    Reference(Option<String>),
}

#[derive(Debug, Default)]
pub struct FindAct {
    pub program: Option<AwardProgram>,
    pub issued_after: Option<DateTime<Utc>>,
    pub operator: Option<String>,
    pub pattern: Option<String>,
    pub group_by: Option<GroupBy>,
    pub log_id: Option<LogId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Default)]
pub struct FindActBuilder {
    pub param: FindAct,
}

impl FindActBuilder {
    pub fn sota(mut self) -> Self {
        self.param.program = Some(AwardProgram::SOTA);
        self
    }

    pub fn pota(mut self) -> Self {
        self.param.program = Some(AwardProgram::POTA);
        self
    }

    pub fn wwff(mut self) -> Self {
        self.param.program = Some(AwardProgram::WWFF);
        self
    }

    pub fn issued_after(mut self, aft: DateTime<Utc>) -> Self {
        self.param.issued_after = Some(aft);
        self
    }

    pub fn limit(mut self, limit: i32) -> Self {
        self.param.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: i32) -> Self {
        self.param.offset = Some(offset);
        self
    }

    pub fn group_by_callsign(mut self, callsign: Option<String>) -> Self {
        self.param.group_by = Some(GroupBy::Callsign(callsign));
        self
    }

    pub fn group_by_reference(mut self, reference: Option<String>) -> Self {
        self.param.group_by = Some(GroupBy::Reference(reference));
        self
    }

    pub fn operator(mut self, operator: &str) -> Self {
        self.param.operator = Some(operator.to_string());
        self
    }

    pub fn pattern(mut self, pattern: &str) -> Self {
        self.param.pattern = Some(pattern.to_string());
        self
    }

    pub fn log_id(mut self, log_id: &str) -> Self {
        self.param.log_id = Some(LogId::from_str(log_id).unwrap());
        self
    }

    pub fn build(self) -> FindAct {
        self.param
    }
}
#[derive(Debug)]
pub struct DeleteAct {
    pub before: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct FindAprs {
    pub callsign: Option<AprsCallsign>,
    pub reference: Option<String>,
    pub after: Option<DateTime<Utc>>,
}
