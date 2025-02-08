use chrono::{DateTime, Utc};
use derive_new::new;

use crate::model::{id::UserId, AwardProgram};
use crate::model::{pota::POTAReferenceWithLog, sota::SOTAReference};

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
    pub dist: Option<f64>,
    pub bbox: Option<BoundingBox>,
    pub center: Option<CenterRadius>,
    pub min_elev: Option<i32>,
    pub min_area: Option<i32>,
    pub user_id: Option<UserId>,
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

    pub fn dist(mut self, dist: f64) -> Self {
        self.param.dist = Some(dist);
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

    pub fn user_id(mut self, id: UserId) -> Self {
        self.param.user_id = Some(id);
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
    pub sota: Option<Vec<SOTAReference>>,
    pub pota: Option<Vec<POTAReferenceWithLog>>,
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

pub struct DeleteLog {
    pub before: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GroupBy {
    Callsign(Option<String>),
    Reference(Option<String>),
}

#[derive(Default, Debug)]
pub struct FindAct {
    pub program: Option<AwardProgram>,
    pub issued_after: Option<DateTime<Utc>>,
    pub pattern: Option<String>,
    pub group_by: Option<GroupBy>,
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

    pub fn pattern(mut self, pattern: String) -> Self {
        self.param.pattern = Some(pattern);
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

#[derive(Debug)]
pub struct FindAprs {
    pub callsign: Option<String>,
    pub after: Option<DateTime<Utc>>,
}
