mod ahd;
mod darktable_advanced;
mod libraw_advanced;
mod linear;
mod ppg;
mod rcd;
mod rust_runner;
mod vng;

pub use ahd::CASE as AHD;
pub use darktable_advanced::{AMAZE, LMMSE};
pub use libraw_advanced::{AAHD, DCB, DHT};
pub use linear::CASE as LINEAR;
pub use ppg::{DARKTABLE_CASE as PPG_DARKTABLE, LIBRAW_CASE as PPG_LIBRAW};
pub use rcd::DARKTABLE_CASE as RCD_DARKTABLE;
pub use vng::{DARKTABLE_CASE as VNG_DARKTABLE, LIBRAW_CASE as VNG_LIBRAW};

use crate::case::Case;

pub const CASES: &[Case] = &[
    AHD,
    AMAZE,
    AAHD,
    DCB,
    DHT,
    LINEAR,
    LMMSE,
    PPG_LIBRAW,
    PPG_DARKTABLE,
    RCD_DARKTABLE,
    VNG_DARKTABLE,
    VNG_LIBRAW,
];
