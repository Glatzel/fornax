use image::Rgb;

/// A trait that defines the behavior of a post-processing operation on decoded
/// image data.
///
/// The `IPostProcessor` trait is intended to be implemented by types that
/// perform post-processing on the data decoded by an `IDecoder`. It allows for
/// the flexibility of applying different post-processing steps (such as
/// filtering, color space transformation, etc.) to the decoded image data.
///
/// # Type Parameters
/// - `D`: The type of the decoder that performs the decoding operation. Must
///   implement `IDecoder<T>`.
/// - `T`: The type of the raw decoded data (e.g., `u8`, `f32`, etc.). Must
///   implement `FornaxPrimitive`.
/// - `O`: The type of the output data after post-processing. Must implement
///   `FornaxPrimitive`.
///
/// # Associated Functions
/// - `post_process(&self, decoder: &D)`: This method is expected to perform the
///   post-processing and return the processed image buffer.
pub trait IPostProcessor<D, T, O>
where
    D: crate::IDecoder<T>,
    T: crate::FornaxPrimitive,
    O: crate::FornaxPrimitive,
{
    /// Perform post-processing on the decoded image data and return a processed
    /// image buffer.
    ///
    /// This method takes a reference to a decoder (`D`) and processes the
    /// decoded data. It returns a `miette::Result` containing an
    /// `image::ImageBuffer` with the post-processed image data.
    fn post_process(
        &self,
        decoder: &D,
    ) -> Result<image::ImageBuffer<Rgb<O>, Vec<O>>, crate::FornaxError>;
}

/// A generic null post-processor that does not perform any post-processing.
///
/// This implementation of the `IPostProcessor` trait does not alter the decoded
/// image data in any way. It is a placeholder implementation to be used when no
/// post-processing is needed.
///
/// assert!(result.is_err()); // Will not return an image as it is
/// unimplemented.
///
/// # Type Parameters
/// - `D`: The type of the decoder that performs the decoding operation. Must
///   implement `IDecoder<u8>`.
///
/// # Notes
/// The `NullPostProcessor` is often used as a no-op processor in cases where no
/// post-processing is required or when setting up a default configuration.
pub struct NullPostProcessor {}
impl<D> IPostProcessor<D, u8, u8> for NullPostProcessor
where
    D: crate::IDecoder<u8>,
{
    fn post_process(
        &self,
        _decoded: &D,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, crate::FornaxError> {
        unimplemented!("Null post processor doesn't return image.")
    }
}
