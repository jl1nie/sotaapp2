use chrono::{DateTime, Utc};
use derive_new::new;

use crate::model::{pota::POTAReference, sota::SOTAReference, AwardProgram};

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

impl<T> From<Vec<T>> for CreateRef<T> {
    fn from(requests: Vec<T>) -> Self {
        Self { requests }
    }
}

#[derive(Default, Debug)]
pub struct FindRef {
    pub program: Vec<AwardProgram>,
    pub ref_id: Option<String>,
    pub name: Option<String>,
    pub bbox: Option<BoundingBox>,
    pub center: Option<CenterRadius>,
    pub min_elev: Option<i32>,
    pub min_area: Option<i32>,
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

    pub fn ref_id(mut self, id: String) -> Self {
        self.param.ref_id = Some(id.to_uppercase());
        self
    }

    pub fn name(mut self, n: String) -> Self {
        self.param.name = Some(n);
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

pub struct CreateRef<T> {
    pub requests: Vec<T>,
}

pub struct UpdateRef<T> {
    pub requests: Vec<T>,
}

#[derive(Debug)]
pub struct FindResult<T> {
    results: Vec<T>,
}

impl<T> FindResult<T> {
    pub fn new(value: Vec<T>) -> Self {
        Self { results: value }
    }

    pub fn get_values(self) -> Option<Vec<T>> {
        (!self.results.is_empty()).then_some(self.results)
    }

    pub fn get_first(self) -> Option<T> {
        self.results.into_iter().next()
    }
}

pub enum ResultKind {
    SOTA(FindResult<SOTAReference>),
    POTA(FindResult<POTAReference>),
}

#[derive(Default)]
pub struct FindAppResult {
    pub results: Vec<ResultKind>,
}
impl FindAppResult {
    pub fn sota(&mut self, v: FindResult<SOTAReference>) {
        self.results.push(ResultKind::SOTA(v))
    }

    pub fn pota(&mut self, v: FindResult<POTAReference>) {
        self.results.push(ResultKind::POTA(v))
    }
}

#[derive(Debug)]
pub enum DeleteRef<T> {
    Delete(T),
    DeleteAll,
}

pub struct UpdateAct<T> {
    pub requests: Vec<T>,
}

impl<T> From<Vec<T>> for UpdateAct<T> {
    fn from(requests: Vec<T>) -> Self {
        Self { requests }
    }
}

#[derive(Default, Debug)]
pub struct FindAct {
    pub program: Option<AwardProgram>,
    pub after: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Default)]
pub struct FindActBuilder {
    pub param: FindAct,
}

impl FindActBuilder {
    pub fn program(mut self, prog: AwardProgram) -> Self {
        self.param.program = Some(prog);
        self
    }
    pub fn after(mut self, aft: DateTime<Utc>) -> Self {
        self.param.after = Some(aft);
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

    pub fn build(self) -> FindAct {
        self.param
    }
}
#[derive(Debug)]
pub struct DeleteAct {
    pub before: DateTime<Utc>,
}
