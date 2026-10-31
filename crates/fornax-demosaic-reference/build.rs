use std::env;
use std::path::PathBuf;

const DARKTABLE_SOURCES: &[&str] = &["amaze.cc", "lmmse.c", "ppg.c", "vng.c"];

fn main() {
    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-env-changed=LIBRAW_ROOT");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = PathBuf::from(env::var_os("CARGO_WORKSPACE_DIR").unwrap());
    for source in DARKTABLE_SOURCES {
        let path = workspace_root
            .join("external/darktable/src/iop/demosaicing")
            .join(source);
        assert!(
            path.is_file(),
            "run the platform setup script to materialize darktable {source}"
        );
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let libraw_root =
        workspace_root.join(env::var("LIBRAW_ROOT").expect("LIBRAW_ROOT must be set"));
    let include_dir = libraw_root.join("include");
    let lib_dir = libraw_root.join("lib");

    let mut reference = cc::Build::new();
    reference
        .cpp(true)
        .warnings(true)
        .include(&include_dir)
        .include(&workspace_root)
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-std=c++17")
        .file(manifest_dir.join("cpp/backend.cpp"));
    reference.compile("fornax_demosaic_reference_cpp");

    let mut darktable_vng = cc::Build::new();
    darktable_vng
        .warnings(true)
        .include(&workspace_root)
        .flag_if_supported("-std=c17")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-parameter")
        .file(manifest_dir.join("cpp/darktable_vng.c"));
    darktable_vng.compile("fornax_demosaic_reference_vng");

    let mut darktable_lmmse = cc::Build::new();
    darktable_lmmse
        .warnings(true)
        .include(&workspace_root)
        .flag_if_supported("-std=c17")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-parameter")
        .file(manifest_dir.join("cpp/darktable_lmmse.c"));
    darktable_lmmse.compile("fornax_demosaic_reference_lmmse");

    let mut darktable_amaze = cc::Build::new();
    darktable_amaze
        .cpp(true)
        .warnings(true)
        .include(manifest_dir.join("cpp/stubs"))
        .include(&workspace_root)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-parameter")
        .file(manifest_dir.join("cpp/darktable_amaze.cc"));
    darktable_amaze.compile("fornax_demosaic_reference_amaze");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=raw_r");
}
