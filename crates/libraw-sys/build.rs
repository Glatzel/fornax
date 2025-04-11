use std::path::PathBuf;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
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
    let libraw_root = match std::env::var("LIBRAW_ROOT") {
        Ok(path) => {
            tracing::info!("Found `LIBRAW_ROOT`: {path}");
            path
        }
        Err(_) => {
            tracing::error!("`LIBRAW_ROOT` not found.");
            panic!("`LIBRAW_ROOT` not found.");
        }
    };
    // Link
    println!("cargo:rustc-link-search=native={libraw_root}/lib");
    println!("cargo:rustc-link-lib=static=jasper");
    tracing::info!("Link to `jasper.lib`");
    println!("cargo:rustc-link-lib=static=jpeg");
    tracing::info!("Link to `lcms2.lib`");
    println!("cargo:rustc-link-lib=static=lcms2");
    tracing::info!("Link to `turbojpeg.lib`");
    println!("cargo:rustc-link-lib=static=turbojpeg");
    tracing::info!("Link to `jpeg.lib`");
    println!("cargo:rustc-link-lib=static=zlib");
    tracing::info!("Link to `zlib.lib`");
    println!("cargo:rustc-link-lib=static=raw_r");
    tracing::info!("Link to `raw_r.lib`");
    #[cfg(feature = "bindgen")]
    {
        let bindings = bindgen::Builder::default()
            .header(format!("{libraw_root}/include/libraw/libraw.h"))
            .use_core()
            .derive_eq(true)
            .ctypes_prefix("libc")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate_comments(true)
            .generate()
            .unwrap();
        let out_file = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");

        bindings
            .write_to_file(&out_file)
            .expect("Couldn't write bindings!");
        tracing::info!("Build bingings to: {}", out_file.to_str().unwrap());
    }
}
