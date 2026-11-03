use fornax_core::BayerPattern;

use crate::error::{DecodeError, HandoffError};
use crate::{CfaPattern, DngMetadata, DngValue};

/// Typed single-plane mosaic samples.
///
/// # Examples
///
/// ```
/// use fornax_decoder::MosaicData;
///
/// let samples = MosaicData::U16(vec![1, 2, 3, 4]);
/// assert_eq!(samples.len(), 4);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum MosaicData {
    /// 8-bit unsigned samples.
    U8(Vec<u8>),
    /// 16-bit unsigned samples.
    U16(Vec<u16>),
    /// 32-bit unsigned samples.
    U32(Vec<u32>),
    /// 32-bit floating-point samples.
    F32(Vec<f32>),
}

impl MosaicData {
    /// Returns the number of samples.
    pub fn len(&self) -> usize {
        match self {
            Self::U8(samples) => samples.len(),
            Self::U16(samples) => samples.len(),
            Self::U32(samples) => samples.len(),
            Self::F32(samples) => samples.len(),
        }
    }

    /// Returns `true` when there are no samples.
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

/// DNG-shaped decoded raw image.
///
/// Pixels remain in their native single-plane mosaic form, and metadata is
/// retained as DNG tags so non-standard CFA layouts can be exported without
/// first being converted to Bayer.
///
/// # Examples
///
/// ```
/// use fornax_decoder::{CfaPattern, DecodedRaw, DngMetadata, MosaicData};
///
/// let image = DecodedRaw::new(
///     2,
///     2,
///     MosaicData::U16(vec![0; 4]),
///     CfaPattern::new(2, 2, vec![0, 1, 1, 2])?,
///     DngMetadata::new(),
/// )?;
/// assert_eq!(image.dimensions(), (2, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw {
    width: u32,
    height: u32,
    pixels: MosaicData,
    cfa: CfaPattern,
    metadata: DngMetadata,
}

impl DecodedRaw {
    /// Creates a decoded image from dimensions, mosaic samples, CFA, and DNG
    /// metadata.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] when a dimension is zero or the
    /// sample count does not match the image dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_decoder::{CfaPattern, DecodedRaw, DngMetadata, MosaicData};
    ///
    /// let image = DecodedRaw::new(
    ///     1,
    ///     1,
    ///     MosaicData::U16(vec![42]),
    ///     CfaPattern::new(2, 2, vec![0, 1, 1, 2])?,
    ///     DngMetadata::new(),
    /// )?;
    /// assert_eq!(image.dimensions(), (1, 1));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(
        width: u32,
        height: u32,
        pixels: MosaicData,
        cfa: CfaPattern,
        metadata: DngMetadata,
    ) -> Result<Self, DecodeError> {
        if width == 0 || height == 0 {
            return Err(DecodeError::InvalidInput {
                detail: format!("image dimensions must not be zero: {width}x{height}").into(),
            });
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| DecodeError::InvalidInput {
                detail: format!("image dimensions overflow: {width}x{height}").into(),
            })?;
        if pixels.len() != expected {
            return Err(DecodeError::InvalidInput {
                detail: format!(
                    "mosaic has {} samples, expected {expected} for {width}x{height}",
                    pixels.len()
                )
                .into(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
            cfa,
            metadata,
        })
    }

    /// Returns the full decoded width and height in pixels.
    pub const fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }

    /// Returns the single-plane mosaic.
    pub const fn pixels(&self) -> &MosaicData { &self.pixels }

    /// Returns the CFA descriptor.
    pub const fn cfa(&self) -> &CfaPattern { &self.cfa }

    /// Returns the DNG metadata container.
    pub const fn metadata(&self) -> &DngMetadata { &self.metadata }

    /// Returns the Bayer pattern accepted by the preprocessor.
    ///
    /// # Errors
    ///
    /// Returns [`HandoffError::UnsupportedCfa`] for every CFA layout other
    /// than a conventional RGB 2x2 Bayer pattern. A missing `CFAPlaneColor`
    /// retains the DNG default RGB mapping; any explicit mapping other than
    /// `[0, 1, 2]` is rejected.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::BayerPattern;
    /// use fornax_decoder::{CfaPattern, DecodedRaw, DngMetadata, MosaicData};
    ///
    /// let image = DecodedRaw::new(
    ///     2,
    ///     2,
    ///     MosaicData::U16(vec![0; 4]),
    ///     CfaPattern::new(2, 2, vec![1, 0, 2, 1])?,
    ///     DngMetadata::new(),
    /// )?;
    /// assert_eq!(image.preprocessor_pattern()?, BayerPattern::GRBG);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn preprocessor_pattern(&self) -> Result<BayerPattern, HandoffError> {
        let rgb_planes = match self.metadata.cfa_plane_colors() {
            None => true,
            Some(DngValue::Byte(colors)) => colors == &[0, 1, 2],
            Some(_) => false,
        };
        if !rgb_planes {
            return Err(HandoffError::UnsupportedCfa(self.cfa.clone()));
        }
        self.cfa
            .bayer_pattern()
            .ok_or_else(|| HandoffError::UnsupportedCfa(self.cfa.clone()))
    }
}
