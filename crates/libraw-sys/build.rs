use std::env;
use std::path::PathBuf;

fn main() {
    let workspace_root = env::var("CARGO_WORKSPACE_DIR").unwrap();
    // run pixi install
    std::process::Command::new("pixi")
        .arg("install")
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to execute script");
    // pkg-config
    #[cfg(target_os = "windows")]
    {
        let path = env::var("PATH").unwrap().to_string();
        let pkg_exe_dir =
            dunce::canonicalize(format!("{workspace_root}/.pixi/envs/default/Library/bin"))
                .unwrap()
                .to_string_lossy()
                .to_string();
        unsafe {
            env::set_var("PATH", format!("{pkg_exe_dir};{path}"));
        }
    }
    let default_pkg_config_path = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "windows" => dunce::canonicalize(format!(
            "{workspace_root}/.pixi/envs/default/libraw/x64-windows-static/lib/pkgconfig"
        ))
        .unwrap()
        .to_string_lossy()
        .to_string(),
        "linux" => dunce::canonicalize(format!(
            "{workspace_root}/.pixi/envs/default/libraw/x64-linux-release/lib/pkgconfig"
        ))
        .unwrap()
        .to_string_lossy()
        .to_string(),
        "macos" => dunce::canonicalize(format!(
            "{workspace_root}/.pixi/envs/default/libraw/arm64-osx-release/lib/pkgconfig"
        ))
        .unwrap()
        .to_string_lossy()
        .to_string(),
        other => {
            panic!("Unsupported OS: {}", other)
        }
    };
    match env::var("PKG_CONFIG_PATH") {
        Ok(var) => unsafe {
            env::set_var(
                "PKG_CONFIG_PATH",
                format!("{var};{}", &default_pkg_config_path),
            );
        },
        Err(_) => unsafe {
            env::set_var("PKG_CONFIG_PATH", &default_pkg_config_path);
        },
    }

    // check LIBCLANG_PATH
    #[cfg(target_os = "windows")]
    match std::env::var("LIBCLANG_PATH") {
        Ok(path) => println!("Found `LIBCLANG_PATH`: {path}"),
        Err(_) => {
            let path = "C:/Program Files/LLVM/bin";

            if std::path::PathBuf::from(path).exists() {
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
    if env::var("UPDATE").unwrap_or("false".to_string()) != "true"
        && env::var("BINDGEN").unwrap_or("false".to_string()) != "true"
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
            .ctypes_prefix("libc")
            .use_core()
            .generate()
            .unwrap();

        bindings
            .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
            .expect("Couldn't write bindings!");
        if env::var("UPDATE").unwrap_or("false".to_string()) == "true" {
            if cfg!(target_os = "windows") {
                bindings
                    .write_to_file("./src/bindings-win.rs")
                    .expect("Couldn't write bindings!");
            }
            if cfg!(target_os = "linux") {
                bindings
                    .write_to_file("./src/bindings-linux.rs")
                    .expect("Couldn't write bindings!");
            }
            if cfg!(target_os = "macos") {
                bindings
                    .write_to_file("./src/bindings-macos.rs")
                    .expect("Couldn't write bindings!");
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
