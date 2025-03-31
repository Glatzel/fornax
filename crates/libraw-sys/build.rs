fn main() {
    unsafe {
        std::env::set_var("VCPKG_ROOT", "./vcpkg/");
    }
    // Use vcpkg toolchain
    let pkg = vcpkg::Config::new()
        .emit_includes(true)
        .find_package("libraw")
        .expect("Failed to find libraw in vcpkg");

    // Get include paths
    let includes = pkg.include_paths.clone();

    // Set up build
    cxx_build::bridge("src/lib.rs")
        .includes(includes)
        .flag_if_supported("-std=c++14")
        .compile("my-vcpkg-crate");

    // Link
    println!("cargo:rustc-link-lib=libraw");
}
