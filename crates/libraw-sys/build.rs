use std::path::PathBuf;
fn main() {
    unsafe {
        std::env::set_var(
            "VCPKG_ROOT",
            "d:\\project\\rs-libraw\\crates\\libraw-sys\\vcpkg",
        );
        std::env::set_var("LIBCLANG_PATH", "C:\\Program Files\\LLVM\\bin");
    }
    // Link
    println!(
        "cargo:rustc-link-search=native=d:\\project\\rs-libraw\\crates\\libraw-sys\\vcpkg\\installed\\x64-windows-static-md\\lib\\manual-link"
    );
    println!("cargo:rustc-link-lib=raw");
    #[cfg(feature = "bindgen")]
    bindings()
}
#[cfg(feature = "bindgen")]
fn bindings() {
    let bindings = bindgen::Builder::default()
        .header("vcpkg/installed/x64-windows-static-md/include/libraw/libraw.h")
        .use_core()
        .derive_eq(true)
        .ctypes_prefix("libc")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_comments(true)
        // Rust doesn't support long double, and bindgen can't skip it
        // // https://github.com/rust-lang/rust-bindgen/issues/1549
        // .blocklist_function("acoshl")
        // .blocklist_function("acosl")
        // .blocklist_function("asinhl")
        // .blocklist_function("asinl")
        // .blocklist_function("atan2l")
        // .blocklist_function("atanhl")
        // .blocklist_function("atanl")
        // .blocklist_function("cbrtl")
        // .blocklist_function("ceill")
        // .blocklist_function("copysignl")
        // .blocklist_function("coshl")
        // .blocklist_function("cosl")
        // .blocklist_function("dreml")
        // .blocklist_function("ecvt_r")
        // .blocklist_function("erfcl")
        // .blocklist_function("erfl")
        // .blocklist_function("exp2l")
        // .blocklist_function("expl")
        // .blocklist_function("expm1l")
        // .blocklist_function("fabsl")
        // .blocklist_function("fcvt_r")
        // .blocklist_function("fdiml")
        // .blocklist_function("finitel")
        // .blocklist_function("floorl")
        // .blocklist_function("fmal")
        // .blocklist_function("fmaxl")
        // .blocklist_function("fminl")
        // .blocklist_function("fmodl")
        // .blocklist_function("frexpl")
        // .blocklist_function("gammal")
        // .blocklist_function("hypotl")
        // .blocklist_function("ilogbl")
        // .blocklist_function("isinfl")
        // .blocklist_function("isnanl")
        // .blocklist_function("j0l")
        // .blocklist_function("j1l")
        // .blocklist_function("jnl")
        // .blocklist_function("ldexpl")
        // .blocklist_function("lgammal")
        // .blocklist_function("lgammal_r")
        // .blocklist_function("llrintl")
        // .blocklist_function("llroundl")
        // .blocklist_function("log10l")
        // .blocklist_function("log1pl")
        // .blocklist_function("log2l")
        // .blocklist_function("logbl")
        // .blocklist_function("logl")
        // .blocklist_function("lrintl")
        // .blocklist_function("lroundl")
        // .blocklist_function("modfl")
        // .blocklist_function("nanl")
        // .blocklist_function("nearbyintl")
        // .blocklist_function("nextafterl")
        // .blocklist_function("nexttoward")
        // .blocklist_function("nexttowardf")
        // .blocklist_function("nexttowardl")
        // .blocklist_function("powl")
        // .blocklist_function("qecvt")
        // .blocklist_function("qecvt_r")
        // .blocklist_function("qfcvt")
        // .blocklist_function("qfcvt_r")
        // .blocklist_function("qgcvt")
        // .blocklist_function("remainderl")
        // .blocklist_function("remquol")
        // .blocklist_function("rintl")
        // .blocklist_function("roundl")
        // .blocklist_function("scalbl")
        // .blocklist_function("scalblnl")
        // .blocklist_function("scalbnl")
        // .blocklist_function("significandl")
        // .blocklist_function("sinhl")
        // .blocklist_function("sinl")
        // .blocklist_function("sqrtl")
        // .blocklist_function("strtold")
        // .blocklist_function("tanhl")
        // .blocklist_function("tanl")
        // .blocklist_function("tgammal")
        // .blocklist_function("truncl")
        // .blocklist_function("y0l")
        // .blocklist_function("y1l")
        // .blocklist_function("ynl")
        .generate()
        .unwrap();
    bindings
        .write_to_file(PathBuf::from("./src/bindings.rs"))
        .expect("Couldn't write bindings!");
}
