use std::io::{Seek, Write};
use std::path::Path;

use fornax_core::MonoImage;

use crate::{
    CfaPatternError, DecodeError, DngMetadata, DngTag, DngValue, DngWriteError, DngWriteOptions,
};

/// Typed single-plane mosaic backed by image-rs buffers.
///
/// # Examples
///
/// ```
/// use fornax_core::MonoImage;
/// use fornax_decoder::MosaicData;
///
/// let samples = MosaicData::U16(MonoImage::from_raw(2, 2, vec![1; 4]).expect("valid mosaic"));
/// assert_eq!(samples.dimensions(), (2, 2));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum MosaicData {
    /// 8-bit unsigned samples.
    U8(MonoImage<u8>),
    /// 16-bit unsigned samples.
    U16(MonoImage<u16>),
    /// 32-bit unsigned samples.
    U32(MonoImage<u32>),
    /// 32-bit floating-point samples.
    F32(MonoImage<f32>),
}

impl MosaicData {
    /// Returns the full decoded width and height in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::U8(image) => image.dimensions(),
            Self::U16(image) => image.dimensions(),
            Self::U32(image) => image.dimensions(),
            Self::F32(image) => image.dimensions(),
        }
    }
}

/// DNG-shaped decoded raw image.
///
/// Pixels remain in a typed image-rs buffer, and all DNG metadata remains in a
/// `BTreeMap` keyed by [`DngTag`]. CFA layout is represented by CFA metadata
/// tags rather than by a duplicate typed field.
///
/// # Examples
///
/// ```
/// use fornax_core::MonoImage;
/// use fornax_decoder::{DecodedRaw, DngMetadata, DngTag, DngValue, MosaicData};
///
/// let pixels = MosaicData::U16(MonoImage::from_raw(2, 2, vec![0; 4]).expect("valid mosaic"));
/// let mut metadata = DngMetadata::new();
/// metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(2));
/// metadata.insert(DngTag::ImageLength, DngValue::Unsigned(2));
///
/// let image = DecodedRaw::new(pixels, metadata)?;
/// assert_eq!(image.dimensions(), (2, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw {
    pixels: MosaicData,
    metadata: DngMetadata,
}

impl DecodedRaw {
    /// Creates a decoded image and validates its DNG dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] when `ImageWidth` or `ImageLength`
    /// is missing, invalid, or disagrees with the mosaic dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::MonoImage;
    /// use fornax_decoder::{DecodedRaw, DngMetadata, DngTag, DngValue, MosaicData};
    ///
    /// let pixels = MosaicData::U16(MonoImage::from_raw(1, 1, vec![42]).expect("valid mosaic"));
    /// let mut metadata = DngMetadata::new();
    /// metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(1));
    /// metadata.insert(DngTag::ImageLength, DngValue::Unsigned(1));
    /// assert!(DecodedRaw::new(pixels, metadata).is_ok());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(pixels: MosaicData, metadata: DngMetadata) -> Result<Self, DecodeError> {
        let (width, height) = pixels.dimensions();
        let tag_width = image_dimension(&metadata, DngTag::ImageWidth)?;
        let tag_height = image_dimension(&metadata, DngTag::ImageLength)?;
        if width != tag_width || height != tag_height {
            return Err(DecodeError::InvalidInput {
                detail: format!(
                    "image dimensions {width}x{height} do not match DNG tags {tag_width}x{tag_height}"
                )
                .into(),
            });
        }
        Ok(Self { pixels, metadata })
    }

    /// Returns the full decoded width and height in pixels.
    pub fn dimensions(&self) -> (u32, u32) { self.pixels.dimensions() }

    /// Returns the typed single-plane mosaic.
    pub const fn pixels(&self) -> &MosaicData { &self.pixels }

    /// Returns the owned DNG metadata map.
    pub const fn metadata(&self) -> &DngMetadata { &self.metadata }

    /// Returns the metadata value for a TIFF/DNG tag.
    pub fn metadata_by_tag(&self, tag: DngTag) -> Option<&DngValue> { self.metadata.get(&tag) }

    /// Returns `BlackLevel`.
    pub fn black_level(&self) -> Option<&DngValue> { self.metadata.get(&DngTag::BlackLevel) }

    /// Returns `CFARepeatPatternDim`.
    ///
    /// # Errors
    ///
    /// Returns [`CfaPatternError`] when the tag has an invalid TIFF value type.
    pub fn cfa_repeat_pattern_dim(&self) -> Result<Option<[u16; 2]>, CfaPatternError> {
        let Some(value) = self.metadata.get(&DngTag::CfaRepeatPatternDim) else {
            return Ok(None);
        };
        let values = short_values(value, DngTag::CfaRepeatPatternDim)?;
        match values.as_slice() {
            [width, height] => Ok(Some([*width, *height])),
            _ => Err(CfaPatternError::LengthMismatch {
                expected: 2,
                actual: values.len(),
            }),
        }
    }

    /// Returns `CFAPattern`.
    ///
    /// # Errors
    ///
    /// Returns [`CfaPatternError`] when the repeat dimensions are missing or
    /// invalid, or when the pattern type or length is invalid.
    pub fn cfa_pattern(&self) -> Result<Option<Vec<u8>>, CfaPatternError> {
        let Some(value) = self.metadata.get(&DngTag::CfaPattern) else {
            return Ok(None);
        };
        let repeat = self
            .cfa_repeat_pattern_dim()?
            .ok_or(CfaPatternError::MissingRepeatPatternDim)?;
        let expected = repeat
            .into_iter()
            .try_fold(1_usize, |product, side| {
                product.checked_mul(usize::from(side))
            })
            .ok_or(CfaPatternError::DimensionsOverflow)?;
        if repeat.contains(&0) {
            return Err(CfaPatternError::Empty);
        }
        let pattern = byte_values(value, DngTag::CfaPattern)?;
        if pattern.len() != expected {
            return Err(CfaPatternError::LengthMismatch {
                expected,
                actual: pattern.len(),
            });
        }
        Ok(Some(pattern))
    }

    /// Returns `CFAPlaneColor`.
    ///
    /// # Errors
    ///
    /// Returns [`CfaPatternError`] when the tag has an invalid TIFF value type.
    pub fn cfa_plane_colors(&self) -> Result<Option<Vec<u8>>, CfaPatternError> {
        let Some(value) = self.metadata.get(&DngTag::CfaPlaneColor) else {
            return Ok(None);
        };
        byte_values(value, DngTag::CfaPlaneColor).map(Some)
    }

    /// Returns `CFALayout`.
    ///
    /// # Errors
    ///
    /// Returns [`CfaPatternError`] when the tag has an invalid TIFF value type
    /// or does not contain exactly one value.
    pub fn cfa_layout(&self) -> Result<Option<u16>, CfaPatternError> {
        let Some(value) = self.metadata.get(&DngTag::CfaLayout) else {
            return Ok(None);
        };
        let values = short_values(value, DngTag::CfaLayout)?;
        match values.as_slice() {
            [layout] => Ok(Some(*layout)),
            _ => Err(CfaPatternError::LengthMismatch {
                expected: 1,
                actual: values.len(),
            }),
        }
    }

    /// Writes this decoded image to a DNG file.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] until DNG writing is
    /// implemented.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::MonoImage;
    /// use fornax_decoder::{DecodedRaw, DngMetadata, DngTag, DngValue, DngWriteOptions, MosaicData};
    ///
    /// let pixels = MosaicData::U16(MonoImage::from_raw(1, 1, vec![0]).expect("valid mosaic"));
    /// let mut metadata = DngMetadata::new();
    /// metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(1));
    /// metadata.insert(DngTag::ImageLength, DngValue::Unsigned(1));
    /// let image = DecodedRaw::new(pixels, metadata)?;
    /// assert!(
    ///     image
    ///         .write_dng("output.dng", DngWriteOptions::default())
    ///         .is_err()
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_dng<P>(&self, _path: P, _options: DngWriteOptions) -> Result<(), DngWriteError>
    where
        P: AsRef<Path>,
    {
        unsupported_write()
    }

    /// Writes this decoded image to a caller-provided DNG byte sink.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] until DNG writing is
    /// implemented.
    pub fn write_dng_with<W>(
        &self,
        _writer: W,
        _options: DngWriteOptions,
    ) -> Result<(), DngWriteError>
    where
        W: Write + Seek,
    {
        unsupported_write()
    }
}

fn image_dimension(metadata: &DngMetadata, tag: DngTag) -> Result<u32, DecodeError> {
    let value = metadata
        .get(&tag)
        .ok_or_else(|| DecodeError::InvalidInput {
            detail: format!("{tag} is missing").into(),
        })?;
    match value {
        DngValue::Short(value) => Ok(u32::from(*value)),
        DngValue::Unsigned(value) => Ok(*value),
        DngValue::UnsignedBig(value) => {
            u32::try_from(*value).map_err(|_| DecodeError::InvalidInput {
                detail: format!("{tag} exceeds u32").into(),
            })
        }
        _ => Err(DecodeError::InvalidInput {
            detail: format!("{tag} has an invalid value type").into(),
        }),
    }
}

fn short_values(value: &DngValue, tag: DngTag) -> Result<Vec<u16>, CfaPatternError> {
    match value {
        DngValue::Short(value) => Ok(vec![*value]),
        DngValue::List(values) => values
            .iter()
            .map(|value| match value {
                DngValue::Short(value) => Ok(*value),
                _ => Err(CfaPatternError::InvalidValueType { tag }),
            })
            .collect(),
        _ => Err(CfaPatternError::InvalidValueType { tag }),
    }
}

fn byte_values(value: &DngValue, tag: DngTag) -> Result<Vec<u8>, CfaPatternError> {
    match value {
        DngValue::Byte(value) => Ok(vec![*value]),
        DngValue::List(values) => values
            .iter()
            .map(|value| match value {
                DngValue::Byte(value) => Ok(*value),
                _ => Err(CfaPatternError::InvalidValueType { tag }),
            })
            .collect(),
        _ => Err(CfaPatternError::InvalidValueType { tag }),
    }
}

fn unsupported_write() -> Result<(), DngWriteError> {
    Err(DngWriteError::Unsupported {
        operation: "DNG writing".into(),
    })
}
