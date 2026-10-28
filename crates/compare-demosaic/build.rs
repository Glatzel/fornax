use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/libraw_shim.cpp");
    println!("cargo:rerun-if-env-changed=LIBRAW_ROOT");

    let libraw_root = PathBuf::from(env::var_os("LIBRAW_ROOT").expect("LIBRAW_ROOT must be set"));
    let include_dir = libraw_root.join("include");
    let lib_dir = libraw_root.join("lib");

    cc::Build::new()
        .cpp(true)
        .file("src/libraw_shim.cpp")
        .include(include_dir)
        .flag_if_supported("-std=c++17")
        .warnings(true)
        .compile("compare_demosaic_libraw_shim");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=raw_r");
}
