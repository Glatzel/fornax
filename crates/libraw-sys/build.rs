#[allow(unused_imports)]
use std::collections::HashSet;
#[allow(unused_imports)]
use std::path::PathBuf;

fn main() {
    // check LIBCLANG_PATH
    #[cfg(target_os = "windows")]
    match std::env::var("LIBCLANG_PATH") {
        Ok(path) => println!("Found `LIBCLANG_PATH`: {path}"),
        Err(_) => {
            let path = "C:/Program Files/LLVM/bin";

            if PathBuf::from(path).exists() {
                unsafe {
                    std::env::set_var("LIBCLANG_PATH", path);
                }
                println!("Set `LIBCLANG_PATH` to: {path}")
            } else {
                panic!("`LIBCLANG_PATH` not found.");
            }
        }
    };

    // Link
    let _pk_libraw = link_lib("libraw_r", "raw_r");
    // generate bindings
    #[cfg(any(feature = "bindgen", feature = "update"))]
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

        let header = &_pk_libraw.include_paths[0]
            .join("libraw/libraw.h")
            .to_string_lossy()
            .to_string();

        let bindings = bindgen::Builder::default()
            .header(header)
            .size_t_is_usize(true)
            .parse_callbacks(Box::new(ignored_macros))
            .generate()
            .unwrap();

        bindings
            .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
            .expect("Couldn't write bindings!");
        #[cfg(feature = "update")]
        bindings
            .write_to_file("./src/bindings.rs")
            .expect("Couldn't write bindings!");
        println!(
            "Build bingings to: {:?}",
            PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs")
        );
    }
}
fn link_lib(name: &str, lib: &str) -> pkg_config::Library {
    match pkg_config::Config::new().probe(name) {
        Ok(pklib) => {
            println!("cargo:rustc-link-lib=static={}", lib);
            println!("Link to `{}`", lib);
            pklib
        }
        Err(e) => panic!("cargo:warning=Pkg-config error: {:?}", e),
    }
}
#[cfg(feature = "bindgen")]
#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);
#[cfg(feature = "bindgen")]
impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}
