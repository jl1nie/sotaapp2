pub mod common;
pub mod pota;
pub mod sota;

#[derive(PartialEq, Debug, sqlx::Type)]
#[repr(i32)]
pub enum AwardProgram {
    SOTA = 0,
    POTA = 1,
    WWFF = 2,
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
