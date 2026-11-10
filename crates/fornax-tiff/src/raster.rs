//! Generic strip and tile raster layout support.

use std::fmt;

use fornax_codec::{ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleFormat};

use crate::constants::TiffTag;
use crate::{Directory, Tag, ValueAccessError, unsigned_vec};

/// TIFF raster storage organization.
///
/// # Examples
///
/// ```
/// assert_ne!(
///     fornax_tiff::RasterStorage::Strips,
///     fornax_tiff::RasterStorage::Tiles
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RasterStorage {
    /// Image rows are stored in strips.
    Strips,
    /// Image samples are stored in rectangular tiles.
    Tiles,
}

/// Validated geometry and encoded chunk locations for a TIFF raster image.
///
/// # Examples
///
/// ```
/// use fornax_tiff::constants::TiffTag;
/// use fornax_tiff::{Directory, RasterLayout, RasterStorage, Tag, Value};
///
/// let mut directory = Directory::default();
/// directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(4));
/// directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(2));
/// directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
/// directory.insert(
///     TiffTag::StripOffsets.into(),
///     Value::List(vec![Value::Unsigned(8), Value::Unsigned(12)]),
/// );
/// directory.insert(
///     TiffTag::StripByteCounts.into(),
///     Value::List(vec![Value::Unsigned(4), Value::Unsigned(4)]),
/// );
///
/// let layout = RasterLayout::from_directory(&directory).unwrap();
/// assert_eq!(layout.storage(), RasterStorage::Strips);
/// assert_eq!((layout.width(), layout.height()), (4, 2));
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterLayout {
    storage: RasterStorage,
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    chunks: Vec<Chunk>,
}

impl RasterLayout {
    /// Reads image and strip/tile geometry from a TIFF directory.
    ///
    /// Tile storage is selected when `TileOffsets` is present. Otherwise strip
    /// tags are required. Chunk locations remain in directory order, which is
    /// the TIFF row-major order.
    ///
    /// # Errors
    ///
    /// Returns [`RasterError`] when a required tag is missing or invalid, a
    /// dimension is zero, chunk lists do not match the image geometry, or
    /// geometry arithmetic overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_tiff::constants::TiffTag;
    /// use fornax_tiff::{Directory, RasterLayout, Value};
    ///
    /// let mut directory = Directory::default();
    /// directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(2));
    /// directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(8));
    /// directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(2));
    ///
    /// assert_eq!(RasterLayout::from_directory(&directory)?.chunks().len(), 1);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub fn from_directory(directory: &Directory) -> Result<Self, RasterError> {
        let width = required_usize(directory, TiffTag::ImageWidth)?;
        let height = required_usize(directory, TiffTag::ImageLength)?;
        if width == 0 {
            return Err(RasterError::ZeroDimension(TiffTag::ImageWidth.into()));
        }
        if height == 0 {
            return Err(RasterError::ZeroDimension(TiffTag::ImageLength.into()));
        }

        let (storage, chunk_width, chunk_height, offsets, byte_counts) =
            if directory.contains_key(&Tag::from(TiffTag::TileOffsets)) {
                let chunk_width = required_usize(directory, TiffTag::TileWidth)?;
                let chunk_height = required_usize(directory, TiffTag::TileLength)?;
                if chunk_width == 0 {
                    return Err(RasterError::ZeroDimension(TiffTag::TileWidth.into()));
                }
                if chunk_height == 0 {
                    return Err(RasterError::ZeroDimension(TiffTag::TileLength.into()));
                }
                (
                    RasterStorage::Tiles,
                    chunk_width,
                    chunk_height,
                    unsigned_usizes(directory, TiffTag::TileOffsets)?,
                    unsigned_usizes(directory, TiffTag::TileByteCounts)?,
                )
            } else {
                let chunk_height = required_usize(directory, TiffTag::RowsPerStrip)?;
                if chunk_height == 0 {
                    return Err(RasterError::ZeroDimension(TiffTag::RowsPerStrip.into()));
                }
                (
                    RasterStorage::Strips,
                    width,
                    chunk_height,
                    unsigned_usizes(directory, TiffTag::StripOffsets)?,
                    unsigned_usizes(directory, TiffTag::StripByteCounts)?,
                )
            };

        if offsets.len() != byte_counts.len() {
            return Err(RasterError::ChunkListLengthMismatch {
                offsets: offsets.len(),
                byte_counts: byte_counts.len(),
            });
        }
        let across = width.div_ceil(chunk_width);
        let down = height.div_ceil(chunk_height);
        let expected = across
            .checked_mul(down)
            .ok_or(RasterError::ChunkCountOverflow)?;
        if offsets.len() != expected {
            return Err(RasterError::ChunkCountMismatch {
                expected,
                found: offsets.len(),
            });
        }
        let chunks = offsets
            .into_iter()
            .zip(byte_counts)
            .map(|(offset, byte_count)| Chunk { offset, byte_count })
            .collect();
        Ok(Self {
            storage,
            width,
            height,
            chunk_width,
            chunk_height,
            chunks,
        })
    }

    /// Returns whether this image uses strips or tiles.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_tiff::constants::TiffTag;
    /// use fornax_tiff::{Directory, RasterLayout, RasterStorage, Value};
    ///
    /// let mut directory = Directory::default();
    /// directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(8));
    /// directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    ///
    /// assert_eq!(
    ///     RasterLayout::from_directory(&directory)?.storage(),
    ///     RasterStorage::Strips,
    /// );
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn storage(&self) -> RasterStorage { self.storage }

    /// Returns the full image width in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterLayout, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(7));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(8));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(7));
    /// assert_eq!(RasterLayout::from_directory(&directory)?.width(), 7);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn width(&self) -> usize { self.width }

    /// Returns the full image height in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterLayout, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(3));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::List(vec![
    /// #     Value::Unsigned(8), Value::Unsigned(9), Value::Unsigned(10)
    /// # ]));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::List(vec![
    /// #     Value::Unsigned(1), Value::Unsigned(1), Value::Unsigned(1)
    /// # ]));
    /// assert_eq!(RasterLayout::from_directory(&directory)?.height(), 3);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn height(&self) -> usize { self.height }

    /// Returns the encoded chunk width in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterLayout, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(3));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(8));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(3));
    /// assert_eq!(RasterLayout::from_directory(&directory)?.chunk_width(), 3);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn chunk_width(&self) -> usize { self.chunk_width }

    /// Returns the encoded chunk height in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterLayout, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(5));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(2));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::List(vec![
    /// #     Value::Unsigned(8), Value::Unsigned(10), Value::Unsigned(12)
    /// # ]));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::List(vec![
    /// #     Value::Unsigned(2), Value::Unsigned(2), Value::Unsigned(1)
    /// # ]));
    /// assert_eq!(RasterLayout::from_directory(&directory)?.chunk_height(), 2);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn chunk_height(&self) -> usize { self.chunk_height }

    /// Returns encoded chunk locations in row-major order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterLayout, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(8));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// let layout = RasterLayout::from_directory(&directory)?;
    /// assert_eq!(layout.chunks()[0].offset, 8);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub fn chunks(&self) -> &[Chunk] { &self.chunks }
}

/// A decoded TIFF raster request with a resolved codec compression.
///
/// This type deliberately accepts an already-resolved codec compression so
/// format crates can apply DNG-specific or vendor-specific ownership rules
/// before constructing the request.
///
/// # Examples
///
/// ```
/// use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
/// use fornax_tiff::constants::TiffTag;
/// use fornax_tiff::{Directory, RasterRequest, Value};
///
/// let mut directory = Directory::default();
/// directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
/// directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
/// directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
/// directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
/// directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
///
/// let request = RasterRequest::new(
///     &[7],
///     &directory,
///     DecodeCompression::Uncompressed,
///     8,
///     SampleFormat::Integer,
///     ByteOrder::Little,
///     Predictor::None,
/// )?;
/// assert_eq!(
///     (request.layout().width(), request.layout().height()),
///     (1, 1)
/// );
/// # Ok::<(), fornax_tiff::RasterError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterRequest<'a> {
    source: &'a [u8],
    compression: DecodeCompression,
    layout: RasterLayout,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    predictor: Predictor,
}

impl<'a> RasterRequest<'a> {
    /// Creates a validated raster request from TIFF directory metadata.
    ///
    /// # Errors
    ///
    /// Returns [`RasterError`] for invalid layout metadata, chunk range
    /// overflow, or chunk ranges outside `source`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// use fornax_tiff::constants::TiffTag;
    /// use fornax_tiff::{Directory, RasterRequest, Value};
    ///
    /// let mut directory = Directory::default();
    /// directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(4));
    /// directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    ///
    /// let request = RasterRequest::new(
    ///     &[0, 0, 0, 0, 9],
    ///     &directory,
    ///     DecodeCompression::Uncompressed,
    ///     8,
    ///     SampleFormat::Integer,
    ///     ByteOrder::Little,
    ///     Predictor::None,
    /// )?;
    /// assert_eq!(request.layout().chunks()[0].offset, 4);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub fn new(
        source: &'a [u8],
        directory: &Directory,
        compression: DecodeCompression,
        bits_per_sample: u16,
        sample_format: SampleFormat,
        byte_order: ByteOrder,
        predictor: Predictor,
    ) -> Result<Self, RasterError> {
        let layout = RasterLayout::from_directory(directory)?;
        for (index, chunk) in layout.chunks().iter().copied().enumerate() {
            let end = chunk
                .offset
                .checked_add(chunk.byte_count)
                .ok_or(RasterError::ChunkRangeOverflow { index })?;
            if end > source.len() {
                return Err(RasterError::ChunkOutOfBounds {
                    index,
                    offset: chunk.offset,
                    byte_count: chunk.byte_count,
                    source_len: source.len(),
                });
            }
        }
        Ok(Self {
            source,
            compression,
            layout,
            bits_per_sample,
            sample_format,
            byte_order,
            predictor,
        })
    }

    /// Returns the resolved codec compression.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// # let request = RasterRequest::new(
    /// #     &[0], &directory, DecodeCompression::Uncompressed, 8,
    /// #     SampleFormat::Integer, ByteOrder::Little, Predictor::None,
    /// # )?;
    /// assert_eq!(request.compression(), DecodeCompression::Uncompressed);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn compression(&self) -> DecodeCompression { self.compression }

    /// Returns the parsed raster layout.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(2));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(2));
    /// # let request = RasterRequest::new(
    /// #     &[0, 0], &directory, DecodeCompression::Uncompressed, 8,
    /// #     SampleFormat::Integer, ByteOrder::Little, Predictor::None,
    /// # )?;
    /// assert_eq!(request.layout().chunks().len(), 1);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn layout(&self) -> &RasterLayout { &self.layout }

    /// Returns the stored bits per sample.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// # let request = RasterRequest::new(
    /// #     &[0], &directory, DecodeCompression::Uncompressed, 12,
    /// #     SampleFormat::Integer, ByteOrder::Little, Predictor::None,
    /// # )?;
    /// assert_eq!(request.bits_per_sample(), 12);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn bits_per_sample(&self) -> u16 { self.bits_per_sample }

    /// Returns the stored sample representation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(4));
    /// # let request = RasterRequest::new(
    /// #     &[0; 4], &directory, DecodeCompression::Uncompressed, 32,
    /// #     SampleFormat::Float, ByteOrder::Little, Predictor::None,
    /// # )?;
    /// assert_eq!(request.sample_format(), SampleFormat::Float);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn sample_format(&self) -> SampleFormat { self.sample_format }

    /// Returns the source byte order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// # let request = RasterRequest::new(
    /// #     &[0], &directory, DecodeCompression::Uncompressed, 8,
    /// #     SampleFormat::Integer, ByteOrder::Big, Predictor::None,
    /// # )?;
    /// assert_eq!(request.byte_order(), ByteOrder::Big);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn byte_order(&self) -> ByteOrder { self.byte_order }

    /// Returns the resolved predictor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// # let request = RasterRequest::new(
    /// #     &[0], &directory, DecodeCompression::Deflate, 8,
    /// #     SampleFormat::Integer, ByteOrder::Little, Predictor::HorizontalInteger,
    /// # )?;
    /// assert_eq!(request.predictor(), Predictor::HorizontalInteger);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub const fn predictor(&self) -> Predictor { self.predictor }

    /// Converts this TIFF raster request into a format-neutral codec request.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax_codec::{ByteOrder, DecodeCompression, Predictor, SampleFormat};
    /// # use fornax_tiff::constants::TiffTag;
    /// # use fornax_tiff::{Directory, RasterRequest, Value};
    /// # let mut directory = Directory::default();
    /// # directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::RowsPerStrip.into(), Value::Unsigned(1));
    /// # directory.insert(TiffTag::StripOffsets.into(), Value::Unsigned(0));
    /// # directory.insert(TiffTag::StripByteCounts.into(), Value::Unsigned(1));
    /// # let request = RasterRequest::new(
    /// #     &[0], &directory, DecodeCompression::Uncompressed, 8,
    /// #     SampleFormat::Integer, ByteOrder::Little, Predictor::None,
    /// # )?;
    /// assert_eq!(request.codec_request().width, 1);
    /// # Ok::<(), fornax_tiff::RasterError>(())
    /// ```
    pub fn codec_request(&self) -> DecodeRequest<'_> {
        DecodeRequest {
            source: self.source,
            compression: self.compression,
            width: self.layout.width,
            height: self.layout.height,
            chunk_width: self.layout.chunk_width,
            chunk_height: self.layout.chunk_height,
            chunks: &self.layout.chunks,
            bits_per_sample: self.bits_per_sample,
            sample_format: self.sample_format,
            byte_order: self.byte_order,
            predictor: self.predictor,
        }
    }
}

/// Failure to read or validate a TIFF raster layout.
///
/// # Examples
///
/// ```
/// let error = fornax_tiff::RasterLayout::from_directory(&Default::default()).unwrap_err();
/// assert!(error.to_string().contains("TIFF tag"));
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RasterError {
    /// A required TIFF tag is absent.
    MissingTag(Tag),
    /// A TIFF tag has a value that cannot be interpreted as an unsigned list.
    InvalidValueType(Tag),
    /// A TIFF numeric value does not fit the platform address space.
    ValueOutOfRange(Tag),
    /// A required TIFF dimension has value zero.
    ZeroDimension(Tag),
    /// The offset and byte-count lists have different lengths.
    ChunkListLengthMismatch {
        /// Number of offsets.
        offsets: usize,
        /// Number of byte counts.
        byte_counts: usize,
    },
    /// Chunk geometry requires a different number of locations.
    ChunkCountMismatch {
        /// Chunk count required by the image geometry.
        expected: usize,
        /// Chunk count in the directory.
        found: usize,
    },
    /// The number of chunks cannot fit the platform address space.
    ChunkCountOverflow,
    /// A chunk's offset plus byte count cannot fit the platform address space.
    ChunkRangeOverflow {
        /// Index in the row-major chunk list.
        index: usize,
    },
    /// A chunk range extends beyond the source buffer.
    ChunkOutOfBounds {
        /// Index in the row-major chunk list.
        index: usize,
        /// Byte offset from the beginning of the source.
        offset: usize,
        /// Encoded byte count.
        byte_count: usize,
        /// Source buffer length.
        source_len: usize,
    },
}

impl fmt::Display for RasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTag(tag) => write!(formatter, "{tag} is missing"),
            Self::InvalidValueType(tag) => {
                write!(formatter, "{tag} has an invalid unsigned value type")
            }
            Self::ValueOutOfRange(tag) => {
                write!(formatter, "{tag} exceeds the platform address space")
            }
            Self::ZeroDimension(tag) => write!(formatter, "{tag} must not be zero"),
            Self::ChunkListLengthMismatch {
                offsets,
                byte_counts,
            } => write!(
                formatter,
                "TIFF chunk offsets and byte counts differ: {offsets} offsets, {byte_counts} byte counts"
            ),
            Self::ChunkCountMismatch { expected, found } => {
                write!(
                    formatter,
                    "TIFF raster geometry requires {expected} chunks, found {found}"
                )
            }
            Self::ChunkCountOverflow => formatter.write_str("TIFF chunk count overflows"),
            Self::ChunkRangeOverflow { index } => {
                write!(formatter, "TIFF chunk {index} range overflows")
            }
            Self::ChunkOutOfBounds {
                index,
                offset,
                byte_count,
                source_len,
            } => write!(
                formatter,
                "TIFF chunk {index} range {offset}..{} is outside source length {source_len}",
                offset.saturating_add(*byte_count)
            ),
        }
    }
}

impl std::error::Error for RasterError {}

/// Failure to resolve a TIFF compression identifier to an available codec.
///
/// # Examples
///
/// ```
/// use fornax_tiff::{TiffCompressionError, resolve_tiff_compression};
///
/// assert_eq!(
///     resolve_tiff_compression(12_345),
///     Err(TiffCompressionError::UnknownCompression(12_345)),
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TiffCompressionError {
    /// The identifier is not registered in the generic TIFF vocabulary.
    UnknownCompression(u16),
    /// The identifier has an implemented codec, but its Cargo feature is off.
    FeatureDisabled {
        /// TIFF compression identifier.
        id: u16,
        /// Cargo feature that enables the implemented codec.
        required_feature: &'static str,
    },
    /// The identifier is known, but Fornax has no codec implementation.
    UnsupportedCompression {
        /// TIFF compression identifier.
        id: u16,
        /// Planned Cargo feature for the missing codec.
        required_feature: &'static str,
    },
}

impl fmt::Display for TiffCompressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCompression(id) => write!(formatter, "unknown TIFF compression {id}"),
            Self::FeatureDisabled {
                id,
                required_feature,
            }
            | Self::UnsupportedCompression {
                id,
                required_feature,
            } => write!(
                formatter,
                "TIFF compression {id} requires codec feature {required_feature}"
            ),
        }
    }
}

impl std::error::Error for TiffCompressionError {}

/// Resolves TIFF-family compression identifiers to codec algorithms.
///
/// Implemented optional codecs whose feature is off return
/// [`TiffCompressionError::FeatureDisabled`]. Known identifiers without a
/// Fornax implementation return
/// [`TiffCompressionError::UnsupportedCompression`]. DNG-owned and
/// vendor-private identifiers are not part of the generic TIFF vocabulary and
/// return [`TiffCompressionError::UnknownCompression`].
///
/// # Examples
///
/// ```
/// use fornax_codec::DecodeCompression;
/// use fornax_tiff::{TiffCompressionError, resolve_tiff_compression};
///
/// assert_eq!(
///     resolve_tiff_compression(1),
///     Ok(DecodeCompression::Uncompressed),
/// );
/// assert_eq!(
///     resolve_tiff_compression(34_892),
///     Err(TiffCompressionError::UnknownCompression(34_892)),
/// );
/// ```
pub const fn resolve_tiff_compression(id: u16) -> Result<DecodeCompression, TiffCompressionError> {
    match id {
        1 => Ok(DecodeCompression::Uncompressed),
        #[cfg(feature = "lzw")]
        5 => Ok(DecodeCompression::Lzw),
        #[cfg(not(feature = "lzw"))]
        5 => Err(TiffCompressionError::FeatureDisabled {
            id,
            required_feature: "lzw",
        }),
        7 => Ok(DecodeCompression::LosslessJpeg),
        8 | 32_946 => Ok(DecodeCompression::Deflate),
        #[cfg(feature = "packbits")]
        32_773 => Ok(DecodeCompression::PackBits),
        #[cfg(not(feature = "packbits"))]
        32_773 => Err(TiffCompressionError::FeatureDisabled {
            id,
            required_feature: "packbits",
        }),
        34_925 => Err(TiffCompressionError::UnsupportedCompression {
            id,
            required_feature: "lzma",
        }),
        #[cfg(feature = "zstd")]
        50_000 => Ok(DecodeCompression::Zstd),
        #[cfg(not(feature = "zstd"))]
        50_000 => Err(TiffCompressionError::FeatureDisabled {
            id,
            required_feature: "zstd",
        }),
        50_001 => Err(TiffCompressionError::UnsupportedCompression {
            id,
            required_feature: "webp",
        }),
        #[cfg(feature = "jpegxl")]
        50_002 => Ok(DecodeCompression::JpegXl),
        #[cfg(not(feature = "jpegxl"))]
        50_002 => Err(TiffCompressionError::FeatureDisabled {
            id,
            required_feature: "jpegxl",
        }),
        _ => Err(TiffCompressionError::UnknownCompression(id)),
    }
}

fn required_usize<T>(directory: &Directory, tag: T) -> Result<usize, RasterError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = directory.get(&tag).ok_or(RasterError::MissingTag(tag))?;
    let value = crate::unsigned_scalar(Some(value)).ok_or(RasterError::InvalidValueType(tag))?;
    usize::try_from(value).map_err(|_| RasterError::ValueOutOfRange(tag))
}

fn unsigned_usizes<T>(directory: &Directory, tag: T) -> Result<Vec<usize>, RasterError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    unsigned_vec::<_, usize>(directory, tag).map_err(|error| match error {
        ValueAccessError::Missing => RasterError::MissingTag(tag),
        ValueAccessError::InvalidType => RasterError::InvalidValueType(tag),
        ValueAccessError::Overflow => RasterError::ValueOutOfRange(tag),
    })
}
