#[cfg(feature = "nef")]
use std::sync::Arc;

use crate::Predictor;
use crate::sample_buffer::SampleBuffer;

/// A raw sample decoding algorithm.
///
/// # Examples
///
/// ```
/// let compression = fornax_codec::DecodeCompression::Uncompressed;
/// assert_eq!(compression, fornax_codec::DecodeCompression::Uncompressed);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeCompression {
    /// Samples are stored directly in the chunk payloads.
    Uncompressed,
    /// TIFF lossless JPEG-92.
    #[cfg(feature = "lossless-jpeg")]
    LosslessJpeg,
    /// TIFF Deflate using zlib framing.
    #[cfg(feature = "deflate")]
    Deflate,
    /// TIFF LZW with MSB-first codes.
    #[cfg(feature = "lzw")]
    Lzw,
    /// TIFF PackBits run-length compression.
    #[cfg(feature = "packbits")]
    PackBits,
    /// TIFF lossy JPEG with one luma component.
    #[cfg(feature = "lossy-jpeg")]
    LossyJpeg,
    /// JPEG XL with one sample channel.
    #[cfg(feature = "jpegxl")]
    JpegXl,
    /// Sony ARW1 difference-coded samples.
    #[cfg(feature = "arw")]
    Arw1,
    /// Sony ARW2 lossy block-coded samples.
    #[cfg(feature = "arw")]
    Arw2 {
        /// Four SonyRawCurve knots, or `None` for the identity curve.
        curve: Option<[u16; 4]>,
    },
    /// Sony ARW6 LLVC3 wavelet-coded samples.
    #[cfg(feature = "arw")]
    Arw6,
    /// Sony lossless-JPEG tiles packed as CFA sample rows.
    #[cfg(feature = "arw")]
    SonyLosslessJpeg,
    /// Canon CRX lossless CFA planes.
    #[cfg(feature = "cr3")]
    CanonCrx {
        /// Validated CMP1 codec header.
        header: CanonCrxHeader,
        /// CFA placement of the decoded mosaic.
        cfa: crate::CanonCrxCfa,
    },
    /// Nikon packed CFA rows with a container-selected row stride.
    #[cfg(feature = "nef")]
    NikonPacked {
        /// Bytes between the starts of adjacent packed rows.
        row_stride: usize,
        /// Stored bits per sample: 12 or 14.
        bits: u8,
        /// Bit and byte order of the packed samples.
        byte_order: ByteOrder,
    },
    /// Nikon 12- or 14-bit samples stored in whole 16-bit words.
    #[cfg(feature = "nef")]
    NikonUnpackedWords {
        /// Stored sample bits: 12 or 14.
        bits: u8,
        /// Byte order of each 16-bit word.
        byte_order: ByteOrder,
    },
    /// Nikon mode-5 stripped data stored in 32-bit little-endian words.
    #[cfg(feature = "nef")]
    NikonStripedPacked12,
    /// Nikon mode-3 lossless Huffman difference coding.
    #[cfg(feature = "nef")]
    NikonHuffman {
        /// Stored sample bits: 12 or 14.
        bits: u8,
        /// Lossless canonical Huffman tree selected by `bits`.
        tree: NikonTreeId,
        /// Initial vertical predictors indexed by row parity and CFA phase.
        init: [[u16; 2]; 2],
        /// Linearization curve indexed by decoded predictor value.
        curve: Arc<[u16]>,
    },
    /// Fujifilm direct-packed CFA rows.
    #[cfg(feature = "raf")]
    FujiPacked {
        /// Stored bits per sample.
        bits: u8,
        /// Byte order used by the packed words.
        byte_order: ByteOrder,
    },
    /// Fujifilm block-based lossless CFA compression.
    #[cfg(feature = "raf")]
    FujiLossless {
        /// CFA pattern used to place decoded line-buffer samples.
        cfa: FujiCfa,
    },
    /// Phase One IIQ uncompressed little-endian samples.
    #[cfg(feature = "iiq")]
    IiqUncompressed,
    /// Phase One IIQ-L and IIQ-L16 row-difference coding.
    #[cfg(feature = "iiq")]
    IiqLossless,
    /// Phase One IIQ-S row-difference coding with nonlinear low-value
    /// expansion.
    #[cfg(feature = "iiq")]
    IiqNonlinear,
    /// Phase One IIQ-Sv2 eight-pixel cluster coding.
    #[cfg(feature = "iiq")]
    IiqSv2,
    /// Panasonic V4 difference-coded samples.
    #[cfg(feature = "rw2")]
    PanasonicV4 {
        /// Whether each 0x4000-byte block uses the split bit-pump layout.
        split_blocks: bool,
    },
    /// Panasonic V5 packetized 12- or 14-bit samples.
    #[cfg(feature = "rw2")]
    PanasonicV5,
    /// Panasonic V6 row packet samples.
    #[cfg(feature = "rw2")]
    PanasonicV6,
    /// Panasonic V8 Huffman, predictor, gamma, and strip-coded samples.
    #[cfg(feature = "rw2")]
    PanasonicV8 {
        /// Validated V8 stream tables and strip placement.
        params: PanasonicV8Params,
    },
    /// Panasonic stamped 12-bit samples in left-aligned little-endian words.
    #[cfg(feature = "rw2")]
    PanasonicUnpacked12,
    /// Panasonic stamped 12-bit samples with one control byte per 16-byte
    /// group.
    #[cfg(feature = "rw2")]
    PanasonicPacked12Control,
    /// TIFF's experimental Zstandard compression.
    #[cfg(feature = "zstd")]
    Zstd,
}

/// Placement and bit extent of one Panasonic V8 compressed strip.
///
/// # Examples
///
/// ```
/// use fornax_codec::PanasonicV8Strip;
///
/// let strip = PanasonicV8Strip::new(0, 64, 8, 4, 32, 16).unwrap();
/// assert_eq!(strip.width(), 32);
/// ```
#[cfg(feature = "rw2")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PanasonicV8Strip {
    pub(crate) source_offset: usize,
    pub(crate) data_bits: u32,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

#[cfg(feature = "rw2")]
impl PanasonicV8Strip {
    /// Creates one validated V8 strip descriptor.
    ///
    /// `source_offset` is relative to [`DecodeRequest::source`], `data_bits`
    /// is the compressed bit count, and the remaining values place the strip
    /// in the full-resolution output.
    ///
    /// # Errors
    ///
    /// Returns [`crate::CodecError::InvalidInput`] when the bit extent is zero,
    /// the strip dimensions are not positive and even, or the placement
    /// arithmetic overflows.
    pub fn new(
        source_offset: usize,
        data_bits: u32,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<Self, crate::CodecError> {
        if data_bits == 0 {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 strip bit count must not be zero".into(),
            ));
        }
        if width == 0 || height == 0 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 strip dimensions must be positive and even".into(),
            ));
        }
        x.checked_add(width)
            .and_then(|_| y.checked_add(height))
            .ok_or_else(|| {
                crate::CodecError::InvalidInput(
                    "Panasonic V8 strip placement overflows usize".into(),
                )
            })?;
        Ok(Self {
            source_offset,
            data_bits,
            x,
            y,
            width,
            height,
        })
    }

    /// Returns the strip's byte offset in the decode source.
    pub const fn source_offset(self) -> usize { self.source_offset }

    /// Returns the strip's compressed bit count.
    pub const fn data_bits(self) -> u32 { self.data_bits }

    /// Returns the strip's horizontal output origin.
    pub const fn x(self) -> usize { self.x }

    /// Returns the strip's vertical output origin.
    pub const fn y(self) -> usize { self.y }

    /// Returns the strip's output width.
    pub const fn width(self) -> usize { self.width }

    /// Returns the strip's output height.
    pub const fn height(self) -> usize { self.height }
}

/// Validated Huffman, gamma, predictor, and strip tables for Panasonic V8.
///
/// # Examples
///
/// ```
/// use fornax_codec::{PanasonicV8Params, PanasonicV8Strip};
///
/// let strip = PanasonicV8Strip::new(0, 16, 0, 0, 4, 2)?;
/// let params = PanasonicV8Params::new(
///     vec![65536; 6],
///     vec![0; 6],
///     0xffff,
///     [0; 4],
///     vec![(1, 0)],
///     vec![0],
///     vec![strip],
/// )?;
/// assert_eq!(params.strip_count(), 1);
/// # Ok::<(), fornax_codec::CodecError>(())
/// ```
#[cfg(feature = "rw2")]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PanasonicV8Params {
    pub(crate) gamma_points: [u32; 6],
    pub(crate) gamma_slopes: [u32; 6],
    pub(crate) clip_value: u16,
    pub(crate) initial_predictors: [u16; 4],
    pub(crate) huffman_table: Vec<(u8, u16)>,
    pub(crate) shift_down: Vec<u16>,
    pub(crate) strips: Vec<PanasonicV8Strip>,
}

#[cfg(feature = "rw2")]
impl PanasonicV8Params {
    /// Creates validated Panasonic V8 decoding tables.
    ///
    /// The Huffman table contains `(bit_count, right_padded_symbol)` pairs.
    /// `shift_down` has one entry per Huffman symbol.
    ///
    /// # Errors
    ///
    /// Returns [`crate::CodecError::InvalidInput`] when a table is empty,
    /// exceeds the format's bounded sizes, has inconsistent lengths, contains
    /// an invalid symbol or shift, or contains overlapping strip placements.
    pub fn new(
        gamma_points: Vec<u32>,
        gamma_slopes: Vec<u32>,
        clip_value: u16,
        initial_predictors: [u16; 4],
        huffman_table: Vec<(u8, u16)>,
        shift_down: Vec<u16>,
        strips: Vec<PanasonicV8Strip>,
    ) -> Result<Self, crate::CodecError> {
        let gamma_points: [u32; 6] = gamma_points.try_into().map_err(|values: Vec<u32>| {
            crate::CodecError::InvalidInput(
                format!(
                    "Panasonic V8 requires 6 gamma points, found {}",
                    values.len()
                )
                .into(),
            )
        })?;
        let gamma_slopes: [u32; 6] = gamma_slopes.try_into().map_err(|values: Vec<u32>| {
            crate::CodecError::InvalidInput(
                format!(
                    "Panasonic V8 requires 6 gamma slopes, found {}",
                    values.len()
                )
                .into(),
            )
        })?;
        if clip_value == 0 {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 clip value must not be zero".into(),
            ));
        }
        if huffman_table.is_empty() || huffman_table.len() > 17 {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 Huffman table must contain 1 to 17 entries".into(),
            ));
        }
        if shift_down.len() != huffman_table.len() {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 Huffman and shift tables have different lengths".into(),
            ));
        }
        for &(bits, symbol) in &huffman_table {
            if bits == 0 || bits > 16 {
                return Err(crate::CodecError::InvalidInput(
                    "Panasonic V8 Huffman symbol length must be 1 to 16 bits".into(),
                ));
            }
            let mask = u16::MAX >> (16 - bits);
            if symbol & !mask != 0 {
                return Err(crate::CodecError::InvalidInput(
                    "Panasonic V8 Huffman symbol exceeds its bit length".into(),
                ));
            }
        }
        if shift_down.iter().any(|shift| *shift > 31) {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 shift-down values must fit five bits".into(),
            ));
        }
        if strips.is_empty() || strips.len() > 64 {
            return Err(crate::CodecError::InvalidInput(
                "Panasonic V8 must contain 1 to 64 strips".into(),
            ));
        }
        for (index, strip) in strips.iter().copied().enumerate() {
            for other in &strips[..index] {
                if rectangles_overlap(strip, *other) {
                    return Err(crate::CodecError::InvalidInput(
                        "Panasonic V8 strips overlap".into(),
                    ));
                }
            }
        }
        Ok(Self {
            gamma_points,
            gamma_slopes,
            clip_value,
            initial_predictors,
            huffman_table,
            shift_down,
            strips,
        })
    }

    /// Returns the number of compressed strips.
    pub const fn strip_count(&self) -> usize { self.strips.len() }

    /// Returns the validated strip descriptors in stream order.
    pub fn strips(&self) -> &[PanasonicV8Strip] { &self.strips }
}

#[cfg(feature = "rw2")]
const fn rectangles_overlap(left: PanasonicV8Strip, right: PanasonicV8Strip) -> bool {
    left.x < right.x + right.width
        && right.x < left.x + left.width
        && left.y < right.y + right.height
        && right.y < left.y + left.height
}

/// A rectangular RGB CFA accepted by the Fujifilm lossless codec.
///
/// # Examples
///
/// ```
/// use fornax_codec::FujiCfa;
///
/// assert_eq!(FujiCfa::Bayer([0, 1, 1, 2]).width(), 2);
/// assert_eq!(FujiCfa::XTrans([1; 36]).height(), 6);
/// ```
#[cfg(feature = "raf")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FujiCfa {
    /// Conventional 2x2 RGB Bayer pattern in row-major order.
    Bayer([u8; 4]),
    /// X-Trans 6x6 RGB pattern in row-major order.
    XTrans([u8; 36]),
}

#[cfg(feature = "raf")]
impl FujiCfa {
    /// Returns the CFA repeat width.
    pub const fn width(self) -> usize {
        match self {
            Self::Bayer(_) => 2,
            Self::XTrans(_) => 6,
        }
    }

    /// Returns the CFA repeat height.
    pub const fn height(self) -> usize {
        match self {
            Self::Bayer(_) => 2,
            Self::XTrans(_) => 6,
        }
    }

    pub(crate) const fn color_at(self, row: usize, column: usize) -> u8 {
        match self {
            Self::Bayer(pattern) => pattern[(row % 2) * 2 + column % 2],
            Self::XTrans(pattern) => pattern[(row % 6) * 6 + column % 6],
        }
    }
}

/// Nikon lossless Huffman tree identifiers.
///
/// # Examples
///
/// ```
/// use fornax_codec::NikonTreeId;
///
/// assert_eq!(NikonTreeId::Lossless12.bits(), 12);
/// ```
#[cfg(feature = "nef")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NikonTreeId {
    /// The canonical tree for 12-bit lossless NEF mode 3.
    Lossless12,
    /// The canonical tree for 14-bit lossless NEF mode 3.
    Lossless14,
}

#[cfg(feature = "nef")]
impl NikonTreeId {
    /// Returns the sample depth associated with this tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::NikonTreeId;
    ///
    /// assert_eq!(NikonTreeId::Lossless14.bits(), 14);
    /// ```
    pub const fn bits(self) -> u8 {
        match self {
            Self::Lossless12 => 12,
            Self::Lossless14 => 14,
        }
    }
}

/// A raw sample encoding algorithm.
///
/// # Examples
///
/// ```
/// let compression = fornax_codec::EncodeCompression::Uncompressed;
/// assert_eq!(compression, fornax_codec::EncodeCompression::Uncompressed);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeCompression {
    /// Samples are stored directly in the chunk payloads.
    Uncompressed,
    /// TIFF lossless JPEG-92.
    #[cfg(feature = "lossless-jpeg")]
    LosslessJpeg,
    /// TIFF Deflate using zlib framing.
    #[cfg(feature = "deflate")]
    Deflate,
    /// TIFF LZW with MSB-first codes.
    #[cfg(feature = "lzw")]
    Lzw,
    /// TIFF PackBits run-length compression.
    #[cfg(feature = "packbits")]
    PackBits,
    /// TIFF's experimental Zstandard compression with an encoding level.
    #[cfg(feature = "zstd")]
    Zstd {
        /// Compression level passed to the Zstandard encoder.
        level: i32,
    },
}

/// The encoded sample's numeric representation.
///
/// # Examples
///
/// ```
/// assert_ne!(
///     fornax_codec::SampleFormat::Integer,
///     fornax_codec::SampleFormat::Float
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    /// Unsigned integer samples.
    Integer,
    /// IEEE-compatible floating-point samples.
    Float,
}

/// Byte order used by direct sample storage.
///
/// # Examples
///
/// ```
/// assert_ne!(
///     fornax_codec::ByteOrder::Little,
///     fornax_codec::ByteOrder::Big
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

/// Canon CRX CMP1 parameters accepted by the lossless decoder.
///
/// The constructor accepts only the audited lossless shape. C-RAW and any
/// ambiguous CMP1 encoding return an unsupported error.
#[cfg(feature = "cr3")]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CanonCrxHeader {
    cmp1_version: u16,
    mosaic_width: u32,
    mosaic_height: u32,
    tile_width: u32,
    tile_height: u32,
    bit_depth: u8,
    encoding_type: u8,
    transform_levels: u8,
    mdat_header_size: u32,
    median_precision: u8,
}

#[cfg(feature = "cr3")]
impl CanonCrxHeader {
    /// Maximum decoded mosaic pixels accepted before allocation.
    pub const MAX_PIXELS: u64 = 120_000_000;

    /// Validates CMP1 fields for the lossless CRX path.
    ///
    /// # Errors
    ///
    /// Returns [`crate::CodecError::Unsupported`] for C-RAW, wavelet-coded,
    /// or unobserved CMP1 shapes and [`crate::CodecError::InvalidInput`] for
    /// zero or oversized geometry.
    #[allow(
        clippy::too_many_arguments,
        reason = "the constructor mirrors the validated CMP1 fields"
    )]
    pub fn new(
        cmp1_version: u16,
        mosaic_width: u32,
        mosaic_height: u32,
        tile_width: u32,
        tile_height: u32,
        bit_depth: u8,
        encoding_type: u8,
        transform_levels: u8,
        mdat_header_size: u32,
        median_precision: u8,
    ) -> Result<Self, crate::CodecError> {
        let pixels = u64::from(mosaic_width) * u64::from(mosaic_height);
        if mosaic_width == 0 || mosaic_height == 0 || tile_width == 0 || tile_height == 0 {
            return Err(crate::error::invalid("Canon CRX geometry must not be zero"));
        }
        if pixels > Self::MAX_PIXELS {
            return Err(crate::error::invalid("Canon CRX mosaic exceeds pixel cap"));
        }
        if !(8..=16).contains(&bit_depth) || median_precision != bit_depth {
            return Err(crate::error::invalid(
                "Canon CRX lossless precision must be 8..=16 bits",
            ));
        }
        if cmp1_version != 0x0100
            || encoding_type != 0
            || transform_levels != 0
            || mdat_header_size != 0x70
        {
            return Err(crate::CodecError::Unsupported(
                "Canon CRX CMP1 shape is not the validated lossless layout".into(),
            ));
        }
        Ok(Self {
            cmp1_version,
            mosaic_width,
            mosaic_height,
            tile_width,
            tile_height,
            bit_depth,
            encoding_type,
            transform_levels,
            mdat_header_size,
            median_precision,
        })
    }

    /// Returns the CMP1 codec version.
    pub const fn cmp1_version(&self) -> u16 { self.cmp1_version }

    /// Returns the full mosaic width.
    pub const fn mosaic_width(&self) -> u32 { self.mosaic_width }

    /// Returns the full mosaic height.
    pub const fn mosaic_height(&self) -> u32 { self.mosaic_height }

    /// Returns the configured tile width.
    pub const fn tile_width(&self) -> u32 { self.tile_width }

    /// Returns the configured tile height.
    pub const fn tile_height(&self) -> u32 { self.tile_height }

    /// Returns the stored bit depth.
    pub const fn bit_depth(&self) -> u8 { self.bit_depth }

    /// Returns the CMP1 encoding type.
    pub const fn encoding_type(&self) -> u8 { self.encoding_type }

    /// Returns the wavelet transform level count.
    pub const fn transform_levels(&self) -> u8 { self.transform_levels }

    /// Returns the CRX MDAT header size.
    pub const fn mdat_header_size(&self) -> u32 { self.mdat_header_size }

    /// Returns the prediction median precision.
    pub const fn median_precision(&self) -> u8 { self.median_precision }
}

/// One encoded strip or tile within [`DecodeRequest::source`].
///
/// # Examples
///
/// ```
/// let chunk = fornax_codec::Chunk {
///     offset: 8,
///     byte_count: 4,
/// };
/// assert_eq!(chunk.offset + chunk.byte_count, 12);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chunk {
    /// Byte offset from the start of the source.
    pub offset: usize,
    /// Number of encoded bytes in the chunk.
    pub byte_count: usize,
}

/// Format-neutral description of one raw image decode.
///
/// # Examples
///
/// ```
/// let request = fornax_codec::DecodeRequest {
///     source: &[],
///     compression: fornax_codec::DecodeCompression::Uncompressed,
///     width: 1,
///     height: 1,
///     chunk_width: 1,
///     chunk_height: 1,
///     chunks: &[],
///     bits_per_sample: 8,
///     sample_format: fornax_codec::SampleFormat::Integer,
///     byte_order: fornax_codec::ByteOrder::Little,
///     predictor: fornax_codec::Predictor::None,
/// };
/// assert_eq!((request.width, request.height), (1, 1));
/// ```
pub struct DecodeRequest<'a> {
    /// Complete source buffer containing the chunk payloads.
    pub source: &'a [u8],
    /// Compression algorithm used to decode the chunk payloads.
    pub compression: DecodeCompression,
    /// Full image width in samples.
    pub width: usize,
    /// Full image height in samples.
    pub height: usize,
    /// Encoded chunk width in samples.
    pub chunk_width: usize,
    /// Encoded chunk height in samples.
    pub chunk_height: usize,
    /// Row-major strip or tile locations.
    pub chunks: &'a [Chunk],
    /// Stored bits per sample.
    pub bits_per_sample: u16,
    /// Stored sample representation.
    pub sample_format: SampleFormat,
    /// Byte order for direct integer and floating-point storage.
    pub byte_order: ByteOrder,
    /// Reversible predictor applied after decompression.
    pub predictor: Predictor,
}

/// Format-neutral description of one raw image encode.
///
/// # Examples
///
/// ```
/// use fornax_codec::{EncodeCompression, SampleBuffer};
///
/// let image = SampleBuffer::from_u8(1, 1, vec![0]).unwrap();
/// let request = fornax_codec::EncodeRequest {
///     image: &image,
///     compression: EncodeCompression::Uncompressed,
///     chunk_width: 1,
///     chunk_height: 1,
///     byte_order: fornax_codec::ByteOrder::Little,
///     predictor: fornax_codec::Predictor::None,
/// };
/// assert_eq!((request.chunk_width, request.chunk_height), (1, 1));
/// ```
pub struct EncodeRequest<'a> {
    /// Mosaic to encode.
    pub image: &'a SampleBuffer,
    /// Compression algorithm used to encode the chunk payloads.
    pub compression: EncodeCompression,
    /// Output chunk width in samples.
    pub chunk_width: usize,
    /// Output chunk height in samples.
    pub chunk_height: usize,
    /// Byte order for direct integer and floating-point storage.
    pub byte_order: ByteOrder,
    /// Predictor to apply before compression.
    pub predictor: Predictor,
}

/// Independently stored encoded chunks in row-major order.
///
/// # Examples
///
/// ```
/// let encoded = fornax_codec::EncodedChunks {
///     chunks: vec![vec![1, 2]],
/// };
/// assert_eq!(encoded.chunks[0], [1, 2]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedChunks {
    /// Encoded bytes for each strip or tile.
    pub chunks: Vec<Vec<u8>>,
}
