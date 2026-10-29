use std::path::PathBuf;
pub fn raw_file() -> PathBuf { root_dir().join("external/raw-images/images/colorchart-eos-7d.cr2") }
pub fn temp_dir() -> PathBuf { root_dir().join("temp") }
pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
    assert!(root.is_dir());
    root
}
