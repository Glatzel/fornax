use std::path::Path;
pub trait IDecoder {
    fn decode_file(&self, file: &Path) -> miette::Result<()>;
    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()>;
    fn bayer_image(&self) -> miette::Result<crate::BayerImage>;
}
