pub mod activation;
pub mod event;
pub mod id;

#[derive(PartialEq, Debug, sqlx::Type)]
#[repr(i32)]
pub enum AwardProgram {
    SOTA = 0,
    POTA = 1,
    WWFF = 2,
}

pub type Maidenhead = String;

impl AwardProgram {
    pub fn as_i32(&self) -> i32 {
        match self {
            AwardProgram::SOTA => 0,
            AwardProgram::POTA => 1,
            AwardProgram::WWFF => 2,
        }
    }
}

impl From<i32> for AwardProgram {
    fn from(value: i32) -> Self {
        match value {
            0 => AwardProgram::SOTA,
            1 => AwardProgram::POTA,
            2 => AwardProgram::WWFF,
            _ => panic!("Invalid value for AwardProgram"),
        }
    }
}

impl From<AwardProgram> for String {
    fn from(value: AwardProgram) -> Self {
        match value {
            AwardProgram::SOTA => "SOTA".to_string(),
            AwardProgram::POTA => "POTA".to_string(),
            AwardProgram::WWFF => "WWFF".to_string(),
        }
    }
}
