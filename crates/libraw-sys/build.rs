use std::path::PathBuf;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();

    // check LIBCLANG_PATH
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
    link_lib("jasper", "jasper");
    link_lib("libjpeg", "jpeg");
    link_lib("lcms2", "lcms2");
    link_lib("libturbojpeg", "turbojpeg");
    link_lib("zlib", "zlib");
    let _pk_libraw = link_lib("libraw_r", "raw_r");

    // generate bindings
    #[cfg(feature = "bindgen")]
    {
        let bindings = bindgen::Builder::default()
            .header(
                _pk_libraw.include_paths[0]
                    .join("libraw/libraw.h")
                    .to_string_lossy(),
            )
            .use_core()
            .derive_eq(true)
            .ctypes_prefix("libc")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate_comments(true)
            .generate()
            .unwrap();

        bindings
            .write_to_file("src/bindings.rs")
            .expect("Couldn't write bindings!");
        clerk::info!("Build bingings to: src/bindings.rs");
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
