pub mod common;
pub mod pota;
pub mod sota;

#[derive(PartialEq, Debug)]
pub enum AwardProgram {
    SOTA,
    POTA,
    WWFF,
}
