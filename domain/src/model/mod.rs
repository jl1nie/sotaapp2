pub mod common;
pub mod pota;
pub mod sota;

#[derive(PartialEq)]
pub enum AwardProgram {
    SOTA,
    POTA,
    WWFF,
}
