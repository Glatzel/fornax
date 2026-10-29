mod linear;
mod ppg;

pub use linear::CASE as LINEAR;
pub use ppg::{DARKTABLE_CASE as PPG_DARKTABLE, LIBRAW_CASE as PPG_LIBRAW};

use crate::case::Case;

pub const CASES: &[Case] = &[LINEAR, PPG_LIBRAW, PPG_DARKTABLE];
