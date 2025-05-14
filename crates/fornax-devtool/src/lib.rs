use std::path::PathBuf;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn example_setup() {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::TRACE, true))
        .init();
    let outdir = output_dir();
    std::fs::create_dir_all(&outdir).expect("output dir already exists.");
    assert!(outdir.is_dir());
}

pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
    assert!(root.is_dir());
    root
}
pub fn output_dir() -> PathBuf {
    let outdir: PathBuf = root_dir().join("temp/fornax/example");
    outdir
}

pub fn raw_file() -> PathBuf { root_dir().join("external/raw-images/images/colorchart-eos-7d.cr2") }
pub  fn get_example_exe(name: &str) -> PathBuf {
    let root = root_dir();
    let mut exe = root.clone();
    #[cfg(target_os = "windows")]
    exe.push(format!("target/llvm-cov-target/debug/examples/{name}.exe"));
    if !exe.exists() {
        exe = root.clone();
        #[cfg(target_os = "windows")]
        exe.push(format!("target/llvm-cov-target/debug/examples/{name}.exe"));
    }
    exe
}
