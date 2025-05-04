use std::collections::HashSet;
use std::fmt::Debug;
use std::path::PathBuf;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();

    // check LIBCLANG_PATH
    #[cfg(target_os = "windows")]
    match std::env::var("LIBCLANG_PATH") {
        Ok(path) => tracing::info!("Found `LIBCLANG_PATH`: {path}"),
        Err(_) => {
            let path = "C:/Program Files/LLVM/bin";

            if PathBuf::from(path).exists() {
                unsafe {
                    std::env::set_var("LIBCLANG_PATH", path);
                }
                tracing::info!("Set `LIBCLANG_PATH` to: {path}")
            } else {
                tracing::error!("`LIBCLANG_PATH` not found.");
                panic!("`LIBCLANG_PATH` not found.");
            }
        }
    };

    // Link
    #[cfg(target_os = "windows")]
    let _pk_libraw = link_lib("libraw_r", "raw_r");
    #[cfg(target_os = "linux")]
    let _pk_libraw = link_lib("libraw_r", "libraw_r");

    // generate bindings
    #[cfg(feature = "bindgen")]
    {
        let header = &_pk_libraw.include_paths[0]
            .join("libraw/libraw.h")
            .to_string_lossy()
            .to_string();

        let bindings = bindgen::Builder::default()
            .header(header)
            .generate()
            .unwrap();

        bindings
            .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
            .expect("Couldn't write bindings!");
        #[cfg(feature = "update")]
        bindings
            .write_to_file("./src/bindings.rs")
            .expect("Couldn't write bindings!");
        clerk::info!(
            "Build bingings to: {:?}",
            PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs")
        );
    }
}
fn link_lib(name: &str, lib: &str) -> pkg_config::Library {
    match pkg_config::Config::new().probe(name) {
        Ok(pklib) => {
            println!("cargo:rustc-link-lib=static={}", lib);
            clerk::info!("Link to `{}`", lib);
            pklib
        }
        Err(e) => panic!("cargo:warning=Pkg-config error: {:?}", e),
    }
}
#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}
