use std::path::PathBuf;
pub fn raw_file() -> PathBuf {
    root_dir().join("external/raw-images/images/colorchart-5D2-6000K.dng")
}
pub fn temp_dir() -> PathBuf { root_dir().join("temp") }
pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    assert!(root.is_dir());
    root
}
