//! Typed TIFF tags and field constants.

use std::fmt;

use num_enum::{FromPrimitive, IntoPrimitive};
use strum::AsRefStr;

use crate::Tag;

/// A typed standard TIFF or TIFF/EP tag identifier.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, AsRefStr, FromPrimitive, IntoPrimitive)]
pub enum TiffTag {
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
    /// `SubIFDs`.
    SubIfds = 330,
    /// `SampleFormat`.
    SampleFormat = 339,
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

/// TIFF compression identifiers used by DNG images.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum CompressionMethod {
    /// Uncompressed samples.
    Uncompressed = 1,
    /// Lossless JPEG-92.
    LosslessJpeg = 7,
    /// Deflate.
    Deflate = 8,
    /// Lossy JPEG.
    LossyJpeg = 34_892,
    /// Zstandard.
    Zstd = 50_000,
    /// Legacy TIFF identifier for JPEG XL.
    JpegXlLegacy = 50_002,
    /// JPEG XL.
    JpegXl = 52_546,
    /// An unsupported or private compression identifier.
    #[num_enum(catch_all)]
    Unknown(u16),
}

/// TIFF photometric interpretation identifiers used by DNG images.
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
    /// Color filter array.
    Cfa = 32_803,
    /// Linear raw samples.
    LinearRaw = 34_892,
    /// An unsupported or private photometric interpretation.
    #[num_enum(catch_all)]
    Unknown(u16),
}
