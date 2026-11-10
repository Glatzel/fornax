use std::path::PathBuf;

use fornax_raw_core::RawFormat;
pub fn raw_file(format: RawFormat) -> PathBuf {
    match format {
        RawFormat::Dng => root_dir().join("external/raw-images/images/colorchart-5D2-6000K.dng"),
        RawFormat::Cr3
        | RawFormat::Arw
        | RawFormat::Nrw
        | RawFormat::Nef
        | RawFormat::Raf
        | RawFormat::Hasselblad3fr
        | RawFormat::Rw2
        | RawFormat::Iiq => panic!("Format doesn't support: {format}"),
    }
}
pub fn temp_dir() -> PathBuf { root_dir().join("temp") }
pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    assert!(root.is_dir());
    root
}
