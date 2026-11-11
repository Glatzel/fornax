//! Typed TIFF tags and field constants.

use std::fmt;

use num_enum::{FromPrimitive, IntoPrimitive};
use strum::AsRefStr;

use crate::Tag;

/// A typed standard TIFF or TIFF/EP tag identifier.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, AsRefStr, FromPrimitive, IntoPrimitive)]
pub enum TiffTag {
    // TIFF 6.0 tags.
    /// `NewSubfileType`.
    NewSubfileType = 254,
    /// `ImageWidth`.
    ImageWidth = 256,
    /// `ImageLength`.
    ImageLength = 257,
    /// `BitsPerSample`.
    BitsPerSample = 258,
    /// `Compression`.
    Compression = 259,
    /// `PhotometricInterpretation`.
    PhotometricInterpretation = 262,
    /// `Make`.
    Make = 271,
    /// `Model`.
    Model = 272,
    /// `StripOffsets`.
    StripOffsets = 273,
    /// `Orientation`.
    Orientation = 274,
    /// `SamplesPerPixel`.
    SamplesPerPixel = 277,
    // TIFF/EP Exif directory pointer.
    /// `ExifIFDPointer`.
    ExifIfdPointer = 34_665,
    // TIFF/EP Exif tags retained by raw containers.
    /// `MakerNote`.
    MakerNote = 37_500,
    /// `RowsPerStrip`.
    RowsPerStrip = 278,
    /// `StripByteCounts`.
    StripByteCounts = 279,
    /// `PlanarConfiguration`.
    PlanarConfiguration = 284,
    /// `Predictor`.
    Predictor = 317,
    /// `TileWidth`.
    TileWidth = 322,
    /// `TileLength`.
    TileLength = 323,
    /// `TileOffsets`.
    TileOffsets = 324,
    /// `TileByteCounts`.
    TileByteCounts = 325,
    /// `SampleFormat`.
    SampleFormat = 339,
    // TIFF Technical Note 1 extension adopted by TIFF/EP.
    /// `SubIFDs`.
    SubIfds = 330,
    // TIFF/EP tags.
    /// `CFARepeatPatternDim`.
    CfaRepeatPatternDim = 33_421,
    /// `CFAPattern`.
    CfaPattern = 33_422,
    /// An unmodeled standard or private TIFF tag.
    #[num_enum(catch_all)]
    Unknown(u16),
}

impl fmt::Display for TiffTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "TIFF tag 0x{value:04x}"),
            _ => write!(formatter, "{self:?}"),
        }
    }
}

impl From<Tag> for TiffTag {
    fn from(value: Tag) -> Self { Self::from(value.0) }
}

impl From<TiffTag> for Tag {
    fn from(value: TiffTag) -> Self { Self(u16::from(value)) }
}

/// TIFF-family compression identifiers handled by the generic raster path.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum CompressionMethod {
    // TIFF 6.0.
    /// Uncompressed samples.
    Uncompressed = 1,
    /// Lempel-Ziv-Welch compression.
    Lzw = 5,
    // TIFF Technical Note 2.
    /// Lossless JPEG-92.
    LosslessJpeg = 7,
    // Adobe Deflate extension also used by DNG 1.4 and later.
    /// Deflate.
    Deflate = 8,
    // TIFF 6.0.
    /// PackBits run-length compression.
    PackBits = 32_773,
    // Legacy LibTIFF Deflate extension.
    /// Legacy Deflate identifier.
    DeflateLegacy = 32_946,
    // LibTIFF LZMA extension.
    /// LZMA2 compression.
    Lzma = 34_925,
    // LibTIFF self-assigned compression identifiers.
    /// Zstandard compression.
    Zstd = 50_000,
    /// WebP compression.
    WebP = 50_001,
    /// JPEG XL compression.
    JpegXl = 50_002,
    /// An unsupported or private compression identifier.
    #[num_enum(catch_all)]
    Unknown(u16),
}

/// TIFF and TIFF/EP photometric interpretation identifiers.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum PhotometricInterpretation {
    /// Zero is white.
    WhiteIsZero = 0,
    /// Zero is black.
    BlackIsZero = 1,
    /// RGB color.
    Rgb = 2,
    /// Palette color.
    Palette = 3,
    /// Transparency mask.
    TransparencyMask = 4,
    /// Separated color.
    Separated = 5,
    /// YCbCr color.
    YCbCr = 6,
    /// CIE L*a*b* color.
    CieLab = 8,
    // TIFF/EP.
    /// Color filter array.
    Cfa = 32_803,
    /// An unsupported or private photometric interpretation.
    #[num_enum(catch_all)]
    Unknown(u16),
}
