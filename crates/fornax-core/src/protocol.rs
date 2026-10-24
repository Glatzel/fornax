//! Draft global protocols connecting decoding, preprocessing, and
//! demosaicing.
//!
//! Concrete backends and algorithms belong to later implementation tasks; this
//! module fixes the shared stage boundaries they implement.

use thiserror::Error;

use crate::{DecodedRaw, PixelSize, RawImage, RawValidationError, ValidatedMetadata};

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
pub trait RawDecoderBackend: Send + Sync {
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
pub trait RawPreprocessor: Send + Sync {
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

/// A demosaicing backend that reconstructs linear planar RGB.
pub trait DemosaicBackend: Send + Sync {
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
    ) -> Result<PlanarRgbF32, DemosaicError>;
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
    /// use fornax_core::{DecodedRaw, PreprocessError, PreprocessOptions, RawImage, RawPreprocessor};
    ///
    /// struct Unsupported;
    /// impl RawPreprocessor for Unsupported {
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
    pub fn pre_process<P: RawPreprocessor + ?Sized>(
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
    pub fn demosaic<D: DemosaicBackend + ?Sized>(
        &self,
        demosaicer: &D,
        options: &DemosaicOptions,
    ) -> Result<DemosaicOutput, DemosaicError> {
        let output = demosaicer.demosaic(self, options)?;
        if output.dimensions() != self.metadata().stored_geometry().dimensions() {
            return Err(DemosaicError::InvalidInput(
                "output dimensions do not match the source image".into(),
            ));
        }
        Ok(DemosaicOutput::new(self.metadata_arc(), output))
    }
}

/// Tightly packed, channel-major linear RGB `f32` output.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarRgbF32 {
    dimensions: PixelSize,
    samples: Box<[f32]>,
}

impl PlanarRgbF32 {
    /// Creates planar RGB output from channel-major samples.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidGeometry`] when the sample count
    /// does not equal `3 * width * height`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{PixelSize, PlanarRgbF32};
    ///
    /// let image = PlanarRgbF32::try_new(PixelSize::new(1, 1).unwrap(), vec![1.0, 0.5, 0.0]).unwrap();
    /// assert_eq!(image.channel(1), Some(&[0.5][..]));
    /// ```
    pub fn try_new(dimensions: PixelSize, samples: Vec<f32>) -> Result<Self, RawValidationError> {
        let pixels = u64::from(dimensions.width())
            .checked_mul(u64::from(dimensions.height()))
            .and_then(|count| count.checked_mul(3))
            .ok_or(RawValidationError::InvalidGeometry)?;
        if u64::try_from(samples.len()) != Ok(pixels) {
            return Err(RawValidationError::InvalidGeometry);
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(RawValidationError::NonFiniteSample);
        }
        Ok(Self {
            dimensions,
            samples: samples.into_boxed_slice(),
        })
    }

    /// Returns output dimensions.
    pub const fn dimensions(&self) -> PixelSize { self.dimensions }

    /// Returns the channel-major R, G, B samples.
    pub fn samples(&self) -> &[f32] { &self.samples }

    /// Returns one channel plane, where channel 0, 1, and 2 are R, G, and B.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::PlanarRgbF32;
    ///
    /// assert_eq!(
    ///     PlanarRgbF32::try_new(
    ///         fornax_core::PixelSize::new(1, 1).unwrap(),
    ///         vec![0.1, 0.2, 0.3],
    ///     )
    ///     .unwrap()
    ///     .channel(2),
    ///     Some(&[0.3][..])
    /// );
    /// ```
    pub fn channel(&self, channel: usize) -> Option<&[f32]> {
        let pixels =
            u64::from(self.dimensions.width()).checked_mul(u64::from(self.dimensions.height()))?;
        let pixels = usize::try_from(pixels).ok()?;
        let start = channel.checked_mul(pixels)?;
        let end = start.checked_add(pixels)?;
        self.samples.get(start..end)
    }
}

/// A demosaicing output retaining the source image metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct DemosaicOutput {
    metadata: std::sync::Arc<ValidatedMetadata>,
    pixels: PlanarRgbF32,
}

impl DemosaicOutput {
    /// Creates demosaicing output while retaining source metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DemosaicOutput, PixelSize, PlanarRgbF32};
    ///
    /// fn accepts_output(_: &DemosaicOutput) {}
    /// assert_eq!(PixelSize::new(1, 1).unwrap().width(), 1);
    /// ```
    pub const fn new(metadata: std::sync::Arc<ValidatedMetadata>, pixels: PlanarRgbF32) -> Self {
        Self { metadata, pixels }
    }

    /// Returns retained source metadata.
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }

    /// Returns linear planar RGB pixels.
    pub const fn pixels(&self) -> &PlanarRgbF32 { &self.pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_backends_are_object_safe() {
        fn decoder(_: &dyn RawDecoderBackend) {}
        fn preprocessor(_: &dyn RawPreprocessor) {}
        fn demosaicer(_: &dyn DemosaicBackend) {}

        let _: fn(&dyn RawDecoderBackend) = decoder;
        let _: fn(&dyn RawPreprocessor) = preprocessor;
        let _: fn(&dyn DemosaicBackend) = demosaicer;
    }

    #[test]
    fn planar_rgb_has_channel_major_layout() {
        let image = PlanarRgbF32::try_new(
            PixelSize::new(2, 1).unwrap(),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )
        .unwrap();
        assert_eq!(image.channel(0), Some(&[1.0, 2.0][..]));
        assert_eq!(image.channel(1), Some(&[3.0, 4.0][..]));
        assert_eq!(image.channel(2), Some(&[5.0, 6.0][..]));
        assert_eq!(image.channel(3), None);
    }

    #[test]
    fn planar_rgb_rejects_wrong_sample_count() {
        assert_eq!(
            PlanarRgbF32::try_new(PixelSize::new(2, 1).unwrap(), vec![0.0; 5]),
            Err(RawValidationError::InvalidGeometry)
        );
        assert_eq!(
            PlanarRgbF32::try_new(PixelSize::new(1, 1).unwrap(), vec![f32::NAN; 3]),
            Err(RawValidationError::NonFiniteSample)
        );
    }
}
