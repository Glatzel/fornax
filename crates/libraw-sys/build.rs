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
    if env::var("UPDATE_LIBRAW_BINDINGS").is_ok() {
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
            .blocklist_function("memcpy|memmove|memset|memcmp|strlen|bcmp")
            .ctypes_prefix("libc")
            .header(header)
            .parse_callbacks(Box::new(ignored_macros))
            .raw_line("#![allow(clippy::all)]")
            .raw_line("#![allow(clippy::nursery)]")
            .raw_line("#![allow(clippy::pedantic)]")
            .raw_line("#![allow(clippy::restriction)]")
            .raw_line("#![allow(dead_code)]")
            .raw_line("#![allow(non_camel_case_types)]")
            .raw_line("#![allow(non_snake_case)]")
            .raw_line("#![allow(non_upper_case_globals)]")
            .raw_line("#![allow(unexpected_cfgs)]")
            .raw_line("#![allow(unnecessary_transmutes)]")
            .size_t_is_usize(true)
            .use_core()
            .generate()
            .unwrap();

        match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
            "windows" => {
                bindings
                    .write_to_file("./src/bindings-win.rs")
                    .expect("Couldn't write bindings!");
            }
            "linux" => match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
                "x86_64" => {
                    bindings
                        .write_to_file("./src/bindings-linux.rs")
                        .expect("Couldn't write bindings!");
                }
                "aarch64" => {
                    bindings
                        .write_to_file("./src/bindings-linux-aarch64.rs")
                        .expect("Couldn't write bindings!");
                }
                other => {
                    panic!("Unsupported OS: {other}")
                }
            },
            "macos" => {
                bindings
                    .write_to_file("./src/bindings-macos.rs")
                    .expect("Couldn't write bindings!");
            }
            other => {
                panic!("Unsupported OS: {other}")
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
