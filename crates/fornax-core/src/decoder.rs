use std::path::Path;

/// A trait that defines the behavior of a decoder for image data.
///
/// The `IDecoder` trait is intended to be implemented by types that decode image data from various
/// sources, such as files or memory buffers. It provides methods for decoding and retrieving raw
/// Bayer image data.
///
/// # Type Parameters
/// - `T`: The type of the decoded data (e.g., `u8`, `f32`, etc.). Must implement `FornaxPrimitive`.
///
/// # Associated Functions
/// - `decode_file(&self, file: &Path)`: Decodes an image file from the given file path.
/// - `decode_buffer(&self, buffer: &[u8])`: Decodes an image from a byte buffer.
/// - `bayer_image(&self)`: Returns the decoded Bayer image data.
pub trait IDecoder<T>
where
    T: crate::FornaxPrimitive,
{
    fn decode_file(&self, file: &Path) -> miette::Result<()>;
    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()>;
    fn bayer_image(&self) -> miette::Result<crate::BayerImage<T>>;
}
