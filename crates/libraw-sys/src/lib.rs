#![allow(
    clippy::approx_constant,
    clippy::redundant_static_lifetimes,
    improper_ctypes
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
)]
#![no_std]

#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
#[cfg(not(feature = "bindgen"))]
mod bindings;
#[cfg(not(feature = "bindgen"))]
pub use bindings::*;
