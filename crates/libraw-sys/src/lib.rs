#![no_std]
#![allow(
    clippy::approx_constant,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
#[cfg(not(feature = "bindgen"))]
include!("bindings.rs");
#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
