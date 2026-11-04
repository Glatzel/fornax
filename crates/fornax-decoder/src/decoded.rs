use std::io::{Seek, Write};
use std::path::Path;

use fornax_core::MonoImage;

use crate::{DecodeError, DngMetadata, DngTag, DngValue, DngWriteError, DngWriteOptions};

/// Typed single-plane mosaic backed by image-rs buffers.
///
/// # Examples
///
/// ```no_run
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
/// ```no_run
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
    pub(crate) metadata: DngMetadata,
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
    /// ```no_run
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
    pub fn new(_pixels: MosaicData, _metadata: DngMetadata) -> Result<Self, DecodeError> { todo!() }

    /// Returns the full decoded width and height in pixels.
    pub fn dimensions(&self) -> (u32, u32) { self.pixels.dimensions() }

    /// Returns the typed single-plane mosaic.
    pub const fn pixels(&self) -> &MosaicData { &self.pixels }

    /// Returns the owned DNG metadata map.
    pub const fn metadata(&self) -> &DngMetadata { &self.metadata }

    /// Returns the metadata value for a TIFF/DNG tag.
    pub fn metadata_by_tag(&self, tag: DngTag) -> Option<&DngValue> { self.metadata.get(&tag) }

    /// Writes this decoded image to a DNG file.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] until DNG writing is
    /// implemented.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        todo!()
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
        todo!()
    }
}
