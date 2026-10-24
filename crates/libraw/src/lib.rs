#![allow(
    dead_code,
    reason = "the internal wrapper retains decoder methods for later tasks"
)]

mod c_api;
mod data_structure;
mod extension;

pub use c_api::ProcFlag;
pub use data_structure::*;
