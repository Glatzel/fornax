mod metadata;
mod tag;



pub use metadata::DngMetadata;
pub use tag::DngTag;

pub type DngValue = tiff::decoder::ifd::Value;
