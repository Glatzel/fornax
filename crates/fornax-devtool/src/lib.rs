use std::path::PathBuf;

pub fn example_setup() {
    clerk::init_log_with_level(clerk::LevelFilter::TRACE);
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
    let outdir: PathBuf = root_dir().join("temp/fornax/dnc");
    outdir
}

pub fn output_dir_for(example: &str, producer: &str) -> PathBuf {
    root_dir().join("temp/fornax").join(example).join(producer)
}

pub fn prepare_output_dir(example: &str, producer: &str) -> std::io::Result<PathBuf> {
    let outdir = output_dir_for(example, producer);
    std::fs::create_dir_all(&outdir)?;
    Ok(outdir)
}

pub fn temp_dir() -> PathBuf { root_dir().join("temp") }

pub fn raw_file() -> PathBuf { root_dir().join("external/raw-images/images/colorchart-eos-7d.cr2") }
pub fn get_example_exe(name: &str) -> PathBuf {
    let root = root_dir();
    let mut exe = root;
    if cfg!(windows) {
        exe.push(format!("target/llvm-cov-target/debug/examples/{name}.exe"));
    } else {
        exe.push(format!("target/llvm-cov-target/debug/examples/{name}"));
    }
    exe
}
