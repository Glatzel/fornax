use std::path::{Path, PathBuf};
use std::{env, fs};

const LIBRAW_SOURCE_VERSION: &str = "0.22.3";
const LIBRAW_SOURCE_COMMIT: &str = "cf02d66eb5c19d4e46df7aee0582e47a4918509a";
const DARKTABLE_SOURCE_RELEASE: &str = "5.6.2";
const DARKTABLE_SOURCE_COMMIT: &str = "20891f6c6fa7be995e5bf9dff9d1ee5062051773";
const SHIM_SOURCES: &[&str] = &[
    "cpp/algos/darktable_ppg.cpp",
    "cpp/algos/libraw_linear.cpp",
    "cpp/algos/libraw_ppg.cpp",
    "cpp/shim.cpp",
];

fn main() {
    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-env-changed=LIBRAW_ROOT");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = PathBuf::from(env::var_os("CARGO_WORKSPACE_DIR").unwrap());
    let darktable_source = workspace_root.join("external/darktable/src/iop/demosaicing/ppg.c");
    assert!(
        darktable_source.is_file(),
        "initialize the pinned darktable submodule: git submodule update --init external/darktable"
    );
    println!("cargo:rerun-if-changed={}", darktable_source.display());
    let libraw_root =
        workspace_root.join(env::var("LIBRAW_ROOT").expect("LIBRAW_ROOT must be set"));
    let include_dir = libraw_root.join("include");
    let lib_dir = libraw_root.join("lib");
    let version = format!(
        "{}.{}.{}",
        libraw_sys::LIBRAW_MAJOR_VERSION,
        libraw_sys::LIBRAW_MINOR_VERSION,
        libraw_sys::LIBRAW_PATCH_VERSION
    );
    assert_eq!(
        version, LIBRAW_SOURCE_VERSION,
        "update the comparison source link when the Pixi LibRaw package changes"
    );
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_VERSION={version}");
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_COMMIT={LIBRAW_SOURCE_COMMIT}");
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_DARKTABLE_RELEASE={DARKTABLE_SOURCE_RELEASE}");
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_DARKTABLE_COMMIT={DARKTABLE_SOURCE_COMMIT}");
    write_status_codes(&manifest_dir);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(true)
        .include(&include_dir)
        .include(&workspace_root)
        .flag_if_supported("/std:c++17") // MSVC
        .flag_if_supported("-std=c++17") // gcc/clang
      ;
    for source in SHIM_SOURCES {
        build.file(manifest_dir.join(source));
    }
    build.compile("compare_demosaic_libraw_reference");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=raw_r");
}

fn write_status_codes(manifest_dir: &Path) {
    let header =
        fs::read_to_string(manifest_dir.join("cpp/shim.h")).expect("reading comparison FFI header");
    let mut generated = String::from("// Generated from cpp/shim.h by build.rs.\n");
    let mut count = 0;
    for line in header
        .lines()
        .filter_map(|line| line.trim().strip_prefix("#define COMPARE_DEMOSAIC_STATUS_"))
    {
        let (name, value) = line
            .split_once(' ')
            .expect("comparison status macro must contain a value");
        let name = format!("COMPARE_DEMOSAIC_STATUS_{name}");
        value
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("invalid value for {name}"));
        generated.push_str(&format!("pub const {name}: i32 = {value};\n"));
        count += 1;
    }
    assert_eq!(count, 8, "comparison FFI header must define 8 statuses");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("status_codes.rs");
    fs::write(output, generated).expect("writing generated comparison FFI statuses");
}
