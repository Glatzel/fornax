use std::env;
use std::path::PathBuf;

fn main() {
    // Link
    let libraw_root = PathBuf::from(env::var("LIBRAW_ROOT").expect("LIBRAW_ROOT must be set"));
    let lib_dir = libraw_root.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=raw_r");

    // #[cfg(target_os = "linux")]
    // {
    //     println!("cargo:rustc-link-lib=m");
    //     println!("cargo:rustc-link-lib=stdc++");
    //     println!("cargo:rustc-link-lib=gomp");
    // }
    // #[cfg(target_os = "macos")]
    // {
    //     println!("cargo:rustc-link-lib=c++");
    //     println!("cargo:rustc-link-lib=m");
    // }
    // generate bindings
    if env::var("UPDATE").unwrap_or("false".to_string()) == "true"
        || env::var("BINDGEN").unwrap_or("false".to_string()) == "true"
    {
        let ignored_macros = IgnoreMacros(
            vec![
                "FP_INFINITE".into(),
                "FP_NAN".into(),
                "FP_NORMAL".into(),
                "FP_SUBNORMAL".into(),
                "FP_ZERO".into(),
            ]
            .into_iter()
            .collect(),
        );
        let include_dir = libraw_root.join("include");
        let header = include_dir
            .join("libraw/libraw.h")
            .to_string_lossy()
            .to_string();

        let bindings = bindgen::Builder::default()
            .header(header)
            .size_t_is_usize(true)
            .parse_callbacks(Box::new(ignored_macros))
            .ctypes_prefix("libc")
            .use_core()
            .generate()
            .unwrap();

        if env::var("UPDATE").unwrap_or("false".to_string()) == "true" {
            match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
                "windows" => {
                    bindings
                        .write_to_file("./src/bindings-win.rs")
                        .expect("Couldn't write bindings!");
                }
                "linux" => {
                    bindings
                        .write_to_file("./src/bindings-linux.rs")
                        .expect("Couldn't write bindings!");
                }
                "macos" => {
                    bindings
                        .write_to_file("./src/bindings-macos.rs")
                        .expect("Couldn't write bindings!");
                }
                other => {
                    panic!("Unsupported OS: {other}")
                }
            }
            if env::var("BINDGEN").unwrap_or("false".to_string()) == "true" {
                println!("cargo:rustc-cfg=bindgen");
                bindings
                    .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
                    .expect("Couldn't write bindings!");
            }
        }
    }
}

#[derive(Debug)]
struct IgnoreMacros(std::collections::HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}
