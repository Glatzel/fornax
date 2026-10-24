//! Draft global protocols connecting decoding, preprocessing, and
//! demosaicing.
//!
//! Concrete backends and algorithms belong to later implementation tasks; this
//! module fixes the shared stage boundaries they implement.

use image::{ImageBuffer, Rgb};
use thiserror::Error;

use crate::{DecodedRaw, PixelSize, RawImage, RawValidationError, ValidatedMetadata};

/// Linear interleaved RGB `f32` output backed by image-rs.
///
/// # Examples
///
/// ```
/// use fornax_core::RgbImageF32;
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
    /// use fornax_core::RawInput;
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

impl DecodedRaw {
    /// Runs the complete standard preprocessing pass.
    ///
    /// # Errors
    ///
    /// Returns the selected preprocessor's structured error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DecodedRaw, IRawPreprocessor, PreprocessError, PreprocessOptions, RawImage};
    ///
    /// struct Unsupported;
    /// impl IRawPreprocessor for Unsupported {
    ///     fn pre_process(
    ///         &self,
    ///         _input: DecodedRaw,
    ///         _options: &PreprocessOptions,
    ///     ) -> Result<RawImage, PreprocessError> {
    ///         Err(PreprocessError::Unsupported("draft".into()))
    ///     }
    /// }
    ///
    /// assert_eq!(PreprocessOptions::default(), PreprocessOptions);
    /// ```
    pub fn pre_process<P: IRawPreprocessor + ?Sized>(
        self,
        preprocessor: &P,
        options: &PreprocessOptions,
    ) -> Result<RawImage, PreprocessError> {
        preprocessor.pre_process(self, options)
    }
}

impl RawImage {
    /// Runs the selected demosaicing backend.
    ///
    /// # Errors
    ///
    /// Returns the selected backend's structured error.
    pub fn demosaic<D: IDemosaic + ?Sized>(
        &self,
        demosaicer: &D,
        options: &DemosaicOptions,
    ) -> Result<(RgbImageF32, std::sync::Arc<ValidatedMetadata>), DemosaicError> {
        let output = demosaicer.demosaic(self, options)?;
        validate_rgb_output(&output, self.metadata().stored_geometry().dimensions())?;
        Ok((output, self.metadata_arc()))
    }
}

fn validate_rgb_output(output: &RgbImageF32, expected: PixelSize) -> Result<(), DemosaicError> {
    let (width, height) = output.dimensions();
    if width != expected.width() || height != expected.height() {
        return Err(DemosaicError::InvalidInput(
            "output dimensions do not match the source image".into(),
        ));
    }
    let expected_samples = expected
        .sample_count()
        .ok()
        .and_then(|count| count.checked_mul(3))
        .ok_or_else(|| DemosaicError::InvalidInput("output size overflows usize".into()))?;
    if output.as_raw().len() != expected_samples {
        return Err(DemosaicError::InvalidInput(
            "output sample count does not match its dimensions".into(),
        ));
    }
    if output.as_raw().iter().any(|sample| !sample.is_finite()) {
        return Err(DemosaicError::InvalidInput(
            "output contains a non-finite sample".into(),
        ));
    }
    Ok(())
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

    #[test]
    fn rgb_image_uses_image_rs_interleaved_layout() {
        let image = RgbImageF32::from_raw(1, 1, vec![0.1, 0.2, 0.3]).unwrap();
        assert_eq!(image.get_pixel(0, 0), &Rgb([0.1, 0.2, 0.3]));
        assert_eq!(
            validate_rgb_output(&image, PixelSize::new(1, 1).unwrap()),
            Ok(())
        );
    }

    #[test]
    fn rgb_output_validation_rejects_invalid_buffers() {
        let wrong_size = RgbImageF32::from_raw(2, 1, vec![0.0; 6]).unwrap();
        assert!(matches!(
            validate_rgb_output(&wrong_size, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
        let wrong_count = RgbImageF32::from_raw(1, 1, vec![0.0; 4]).unwrap();
        assert!(matches!(
            validate_rgb_output(&wrong_count, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
        let non_finite = RgbImageF32::from_raw(1, 1, vec![f32::NAN; 3]).unwrap();
        assert!(matches!(
            validate_rgb_output(&non_finite, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
    }
}
