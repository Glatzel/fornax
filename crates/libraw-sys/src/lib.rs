#![allow(
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
#[cfg(all(not(feature = "bindgen"), target_os = "windows"))]
include!("bindings-win.rs");
#[cfg(all(not(feature = "bindgen"), target_os = "linux"))]
include!("bindings-linux.rs");
#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
