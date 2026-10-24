//! Draft global protocols connecting decoding, preprocessing, and
//! demosaicing.
//!
//! Concrete backends and algorithms belong to later implementation tasks; this
//! module fixes the shared stage boundaries they implement.

use image::{ImageBuffer, Rgb};
use thiserror::Error;

use crate::{DecodedRaw, RawImage, RawValidationError};

/// Linear interleaved RGB `f32` output backed by image-rs.
///
/// # Examples
///
/// ```
/// use fornax::RgbImageF32;
///
/// let image = RgbImageF32::from_raw(1, 1, vec![0.1, 0.2, 0.3]).unwrap();
/// assert_eq!(image.as_raw(), &[0.1, 0.2, 0.3]);
/// ```
pub type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

/// Raw bytes and optional source name supplied to a decoder backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RawInput<'a> {
    bytes: &'a [u8],
    source_name: Option<&'a str>,
}

impl<'a> RawInput<'a> {
    /// Creates decoder input over an in-memory RAW file.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::RawInput;
    ///
    /// let input = RawInput::new(b"raw bytes", Some("image.dng"));
    /// assert_eq!(input.source_name(), Some("image.dng"));
    /// ```
    pub const fn new(bytes: &'a [u8], source_name: Option<&'a str>) -> Self {
        Self { bytes, source_name }
    }

    /// Returns the complete input bytes.
    pub const fn bytes(self) -> &'a [u8] { self.bytes }

    /// Returns the optional source filename for provenance and diagnostics.
    pub const fn source_name(self) -> Option<&'a str> { self.source_name }
}

/// Draft decoder configuration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DecodeConfig;

/// Draft preprocessing configuration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PreprocessOptions;

/// Draft demosaicing configuration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DemosaicOptions;

/// Structured decoder backend failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    /// The input container or file cannot be parsed.
    #[error("invalid RAW input: {0}")]
    InvalidInput(Box<str>),
    /// The input format or layout is unsupported by this backend.
    #[error("unsupported RAW format or layout: {0}")]
    Unsupported(Box<str>),
    /// The backend failed after recognizing the input.
    #[error("decoder backend failure: {0}")]
    Backend(Box<str>),
    /// The backend produced metadata or samples that fail core validation.
    #[error(transparent)]
    InvalidOutput(#[from] RawValidationError),
}

/// Structured preprocessing failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PreprocessError {
    /// The decoded image cannot enter standard preprocessing.
    #[error("invalid preprocessing input: {0}")]
    InvalidInput(Box<str>),
    /// The configured preprocessing operation is unsupported.
    #[error("unsupported preprocessing operation: {0}")]
    Unsupported(Box<str>),
    /// Floating-point arithmetic produced an invalid value.
    #[error("invalid preprocessing numerical result")]
    Numeric,
    /// A required preprocessing allocation failed.
    #[error("preprocessing allocation failed")]
    Allocation,
}

/// Structured demosaicing failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DemosaicError {
    /// The preprocessed image or requested output dimensions are invalid.
    #[error("invalid demosaicing input: {0}")]
    InvalidInput(Box<str>),
    /// The selected strategy does not support the image or configuration.
    #[error("unsupported demosaicing operation: {0}")]
    Unsupported(Box<str>),
    /// The strategy failed after accepting the input.
    #[error("demosaicing strategy failure: {0}")]
    Strategy(Box<str>),
    /// A required demosaicing allocation failed.
    #[error("demosaicing allocation failed")]
    Allocation,
}

/// A decoder backend that produces the universal decoded representation.
pub trait IRawDecoder: Send + Sync {
    /// Decodes one RAW input into a validated [`DecodedRaw`].
    ///
    /// # Errors
    ///
    /// Returns a structured [`DecodeError`] for malformed, unsupported, or
    /// backend-specific failures.
    fn decode(
        &self,
        input: &RawInput<'_>,
        config: &DecodeConfig,
    ) -> Result<DecodedRaw, DecodeError>;
}

/// A preprocessing backend that performs the complete standard RAW pass.
pub trait IRawPreprocessor: Send + Sync {
    /// Converts one decoded image into a demosaic-ready f32-only [`RawImage`].
    ///
    /// # Errors
    ///
    /// Returns a structured [`PreprocessError`] for invalid input, numeric, or
    /// allocation failures.
    fn pre_process(
        &self,
        input: DecodedRaw,
        options: &PreprocessOptions,
    ) -> Result<RawImage, PreprocessError>;
}

/// A demosaicing backend that reconstructs linear interleaved RGB.
pub trait IDemosaic: Send + Sync {
    /// Reconstructs linear RGB from one preprocessed Bayer image.
    ///
    /// # Errors
    ///
    /// Returns a structured [`DemosaicError`] for unsupported input,
    /// strategy-specific failure, or allocation failure.
    fn demosaic(
        &self,
        input: &RawImage,
        options: &DemosaicOptions,
    ) -> Result<RgbImageF32, DemosaicError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_backends_are_object_safe() {
        fn decoder(_: &dyn IRawDecoder) {}
        fn preprocessor(_: &dyn IRawPreprocessor) {}
        fn demosaicer(_: &dyn IDemosaic) {}

        let _: fn(&dyn IRawDecoder) = decoder;
        let _: fn(&dyn IRawPreprocessor) = preprocessor;
        let _: fn(&dyn IDemosaic) = demosaicer;
    }
}
