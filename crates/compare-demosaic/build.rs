use std::path::{self, Path, PathBuf};
use std::{env, fs};

const LIBRAW_SOURCE_VERSION: &str = "0.22.3";
const LIBRAW_SOURCE_COMMIT: &str = "cf02d66eb5c19d4e46df7aee0582e47a4918509a";
const SHIM_SOURCES: &[&str] = &["cpp/algos/linear.cpp", "cpp/shim.cpp"];

fn main() {
    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-env-changed=LIBRAW_ROOT");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let libraw_root = find_libraw_root(&manifest_dir);
    let include_dir = libraw_root.join("include");
    let lib_dir = libraw_root.join("lib");
    let version = libraw_version(&include_dir);
    assert_eq!(
        version, LIBRAW_SOURCE_VERSION,
        "update the comparison source link when the Pixi LibRaw package changes"
    );
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_VERSION={version}");
    println!("cargo:rustc-env=COMPARE_DEMOSAIC_LIBRAW_COMMIT={LIBRAW_SOURCE_COMMIT}");
    write_status_codes(&manifest_dir);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .warnings(true)
        .include(&include_dir)
        .flag_if_supported("-std=c++17");
    for source in SHIM_SOURCES {
        build.file(manifest_dir.join(source));
    }
    build.compile("compare_demosaic_libraw_reference");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=raw_r");
    if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
}

fn find_libraw_root(manifest_dir: &Path) -> PathBuf {
    if let Some(root) = env::var_os("LIBRAW_ROOT") {
        return PathBuf::from(root);
    }
    let workspace_root = manifest_dir
        .ancestors()
        .nth(2)
        .expect("compare-demosaic must be inside the workspace");
    let repository_prefix = platform_prefix(&workspace_root.join(".pixi/envs/dev"));
    if repository_prefix.join("include/libraw/libraw.h").is_file() {
        return repository_prefix;
    }
    if let Some(prefix) = env::var_os("CONDA_PREFIX") {
        let prefix = platform_prefix(&PathBuf::from(prefix));
        if prefix.join("include/libraw/libraw.h").is_file() {
            return prefix;
        }
    }
    panic!(
        "LibRaw headers are missing. Run `pixi install -e dev` or set LIBRAW_ROOT to a LibRaw installation."
    );
}

fn platform_prefix(prefix: &Path) -> PathBuf {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        prefix.join("Library")
    } else {
        prefix.to_path_buf()
    }
}

fn libraw_version(include_dir: &Path) -> String {
    let header_path = path::absolute(include_dir.join("libraw/libraw_version.h")).unwrap();
    let header = fs::read_to_string(&header_path).unwrap_or_else(|error| {
        panic!(
            "reading LibRaw version header from the Pixi package at {}: {error}",
            header_path.display()
        )
    });
    let field = |name: &str| {
        header
            .lines()
            .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
            .unwrap_or_else(|| panic!("missing {name} in LibRaw version header"))
            .trim()
    };
    format!(
        "{}.{}.{}",
        field("LIBRAW_MAJOR_VERSION"),
        field("LIBRAW_MINOR_VERSION"),
        field("LIBRAW_PATCH_VERSION")
    )
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
