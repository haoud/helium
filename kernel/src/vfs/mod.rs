use macros::init;

pub mod fs;
pub mod mount;
pub mod name;
pub mod path;

#[init]
pub fn setup() {}
