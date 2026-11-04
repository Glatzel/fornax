use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use fornax_core::MonoImage;

use crate::error::{DecodeError, DngWriteError};
use crate::{DngCompression, DngMetadata, DngTag, DngValue, DngWriteOptions};

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
    pub(crate) source: Option<DngSource>,
}

/// Original container retained for byte-identical default DNG export.
#[derive(Clone, Debug, PartialEq)]
pub struct DngSource {
    pub(crate) bytes: Vec<u8>,
    pub(crate) compression: DngCompression,
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
    pub fn new(pixels: MosaicData, metadata: DngMetadata) -> Result<Self, DecodeError> {
        validate_dimensions(&pixels, &metadata)?;
        Ok(Self {
            pixels,
            metadata,
            source: None,
        })
    }

    pub(crate) fn from_decoded(
        pixels: MosaicData,
        metadata: DngMetadata,
        source: Option<DngSource>,
    ) -> Result<Self, DecodeError> {
        validate_dimensions(&pixels, &metadata)?;
        Ok(Self {
            pixels,
            metadata,
            source,
        })
    }

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
    /// A decoded source is preserved byte for byte when the requested
    /// compression matches its original encoding. Otherwise,
    /// [`DngCompression::Uncompressed`] regenerates a raw CFA directory from
    /// the current samples.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] when lossless JPEG re-encoding
    /// is requested without a matching source container.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_core::MonoImage;
    /// use fornax_decoder::{
    ///     DecodedRaw, DngCompression, DngMetadata, DngTag, DngValue, DngWriteOptions, MosaicData,
    /// };
    ///
    /// let pixels = MosaicData::U16(MonoImage::from_raw(1, 1, vec![0]).expect("valid mosaic"));
    /// let mut metadata = DngMetadata::new();
    /// metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(1));
    /// metadata.insert(DngTag::ImageLength, DngValue::Unsigned(1));
    /// let image = DecodedRaw::new(pixels, metadata)?;
    /// assert!(
    ///     image
    ///         .write_dng(
    ///             "output.dng",
    ///             DngWriteOptions::new(DngCompression::Uncompressed),
    ///         )
    ///         .is_ok()
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_dng<P>(&self, path: P, options: DngWriteOptions) -> Result<(), DngWriteError>
    where
        P: AsRef<Path>,
    {
        let file = File::create(path.as_ref()).map_err(|error| DngWriteError::Io {
            operation: format!("creating {}", path.as_ref().display()).into(),
            detail: error.to_string().into(),
        })?;
        self.write_dng_with(BufWriter::new(file), options)
    }

    /// Writes this decoded image to a caller-provided DNG byte sink.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] when lossless JPEG re-encoding
    /// is requested without a matching source container.
    pub fn write_dng_with<W>(
        &self,
        mut writer: W,
        options: DngWriteOptions,
    ) -> Result<(), DngWriteError>
    where
        W: Write + Seek,
    {
        if let Some(source) = &self.source
            && source.compression == options.compression()
        {
            writer
                .write_all(&source.bytes)
                .and_then(|()| writer.flush())
                .map_err(|error| DngWriteError::Io {
                    operation: "writing preserved DNG".into(),
                    detail: error.to_string().into(),
                })?;
            return Ok(());
        }

        if options.compression() == DngCompression::Uncompressed {
            return crate::writer::write_uncompressed(self, writer, options);
        }

        Err(DngWriteError::Unsupported {
            operation: format!(
                "{:?} DNG export without matching source encoding",
                options.compression()
            )
            .into(),
        })
    }
}

fn validate_dimensions(pixels: &MosaicData, metadata: &DngMetadata) -> Result<(), DecodeError> {
    let width = metadata_u32(metadata, DngTag::ImageWidth)?;
    let height = metadata_u32(metadata, DngTag::ImageLength)?;
    let (actual_width, actual_height) = pixels.dimensions();
    if (width, height) != (actual_width, actual_height) {
        return Err(DecodeError::InvalidInput {
            detail: format!(
                "DNG metadata dimensions are {width}x{height}, mosaic dimensions are \
                 {actual_width}x{actual_height}"
            )
            .into(),
        });
    }
    Ok(())
}

fn metadata_u32(metadata: &DngMetadata, tag: DngTag) -> Result<u32, DecodeError> {
    let value = metadata
        .get(&tag)
        .ok_or_else(|| DecodeError::InvalidInput {
            detail: format!("{tag} is missing").into(),
        })?;
    value
        .clone()
        .into_u32()
        .map_err(|_| DecodeError::InvalidInput {
            detail: format!("{tag} has an invalid value type").into(),
        })
}
