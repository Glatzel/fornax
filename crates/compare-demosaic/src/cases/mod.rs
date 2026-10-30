mod linear;
mod ppg;
mod rcd;

pub use linear::CASE as LINEAR;
pub use ppg::{DARKTABLE_CASE as PPG_DARKTABLE, LIBRAW_CASE as PPG_LIBRAW};
pub use rcd::DARKTABLE_CASE as RCD_DARKTABLE;

use crate::case::Case;

pub const CASES: &[Case] = &[LINEAR, PPG_LIBRAW, PPG_DARKTABLE, RCD_DARKTABLE];
