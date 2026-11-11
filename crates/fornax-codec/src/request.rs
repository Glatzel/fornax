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
    /// TIFF's experimental Zstandard compression.
    #[cfg(feature = "zstd")]
    Zstd,
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
