#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
#[cfg(not(feature = "bindgen"))]
include!("bindings.rs");
#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
