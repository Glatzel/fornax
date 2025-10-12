use std::path::Path;

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use dnc;
pub use fornax_core::NullPostProcessor;
use fornax_core::{FornaxError, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{ImageBuffer, Rgb};
pub use {fornax_dalim, libraw};
/// A struct that integrates decoding and post-processing of image data.
///
/// `Fornax` combines the functionality of an image decoder and a
/// post-processor. It allows for decoding an image from a file or buffer,
/// followed by applying a post-processing operation on the decoded data. It is
/// a generic struct that supports any decoder that implements `IDecoder` and
/// any post-processor that implements `IPostProcessor`.
///
/// # Type Parameters
/// - `D`: The type of the decoder used to decode the image data. It must
///   implement the `IDecoder<T>` trait.
/// - `T`: The type of the raw decoded data (e.g., `u8`, `f32`). It must
///   implement the `FornaxPrimitive` trait.
/// - `P`: The type of the post-processor used to apply post-processing to the
///   decoded data. It must implement the `IPostProcessor<D, T, O>` trait.
/// - `O`: The type of the output data after post-processing (e.g., `u8`,
///   `f32`). It must implement the `FornaxPrimitive` trait.
pub struct Fornax<D, T, P, O>
where
    D: IDecoder<T>,
    T: FornaxPrimitive,
    P: IPostProcessor<D, T, O>,
    O: FornaxPrimitive,
{
    _marker_t: std::marker::PhantomData<T>,
    _marker_o: std::marker::PhantomData<O>,
    pub decoder: D,
    pub post_processor: P,
}

impl<D, T, P, O> Fornax<D, T, P, O>
where
    D: IDecoder<T>,
    T: FornaxPrimitive,
    P: IPostProcessor<D, T, O>,
    O: FornaxPrimitive,
{
    /// Creates a new instance of `Fornax` with the provided decoder and
    /// post-processor.
    ///
    /// # Arguments
    /// - `decoder`: The decoder responsible for decoding image data.
    /// - `post_processor`: The post-processor responsible for processing the
    ///   decoded data.
    ///
    /// # Returns
    /// A new `Fornax` instance with the specified decoder and post-processor.
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            _marker_t: std::marker::PhantomData,
            _marker_o: std::marker::PhantomData,
            decoder,
            post_processor,
        }
    }
    /// Decodes an image file from the given file path.
    ///
    /// This method decodes the image from the specified file and returns a
    /// reference to the `Fornax` instance. It allows chaining further
    /// operations such as post-processing.
    ///
    /// # Arguments
    /// - `file`: The path to the image file to decode.
    ///
    /// # Returns
    /// A `Result` with a reference to the `Fornax` instance if decoding is
    /// successful.
    pub fn decode_file(&self, file: &Path) -> Result<&Self, FornaxError> {
        self.decoder.decode_file(file)?;
        Ok(self)
    }
    /// Decodes an image from a byte buffer.
    ///
    /// This method decodes the image from the given buffer and returns a
    /// reference to the `Fornax` instance. It allows chaining further
    /// operations such as post-processing.
    ///
    /// # Arguments
    /// - `buffer`: A byte slice containing the image data to decode.
    ///
    /// # Returns
    /// A `Result` with a reference to the `Fornax` instance if decoding is
    /// successful.
    pub fn decode_buffer(&self, buffer: &[u8]) -> Result<&Self, FornaxError> {
        self.decoder.decode_buffer(buffer)?;
        Ok(self)
    }
    /// Applies post-processing to the decoded image data.
    ///
    /// This method invokes the post-processor on the decoded data and returns
    /// the processed image buffer.
    ///
    /// # Returns
    /// A `Result` containing the post-processed image buffer if successful.
    pub fn post_process(&self) -> Result<ImageBuffer<Rgb<O>, Vec<O>>, FornaxError> {
        self.post_processor.post_process(&self.decoder)
    }
}
