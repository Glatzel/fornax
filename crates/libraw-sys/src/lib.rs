#![no_std]
#![allow(
    clippy::approx_constant,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unexpected_cfgs,
    unsafe_code
)]
#[cfg(all(not(bindgen), target_os = "windows"))]
include!("bindings-win.rs");
#[cfg(all(not(bindgen), target_os = "linux", target_arch = "x86_64"))]
include!("bindings-linux.rs");
#[cfg(all(not(bindgen), target_os = "linux", target_arch = "aarch64"))]
include!("bindings-linux-aarch64.rs");
#[cfg(all(not(bindgen), target_os = "macos"))]
include!("bindings-macos.rs");

#[cfg(bindgen)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
