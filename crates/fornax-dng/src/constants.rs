//! DNG-owned compression and photometric interpretation values.
//!
//! Compression sources and ownership are documented in
//! [the DNG compression
//! reference](../../../docs/references/dng-compression.md).

use num_enum::{FromPrimitive, IntoPrimitive};

/// Compression identifiers interpreted by the DNG format layer.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum DngCompressionMethod {
    /// DNG lossy JPEG.
    LossyJpeg = 34_892,
    /// LibTIFF's self-assigned Zstandard compatibility identifier.
    Zstd = 50_000,
    /// GDAL's legacy self-assigned JPEG XL compatibility identifier.
    JpegXlLegacy = 50_002,
    /// DNG 1.7 JPEG XL.
    JpegXl = 52_546,
    /// An unsupported or private compression identifier.
    #[num_enum(catch_all)]
    Unknown(u16),
}

/// Photometric interpretation values introduced by the DNG specification.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum DngPhotometricInterpretation {
    /// Linear raw samples.
    LinearRaw = 34_892,
    /// Depth-map samples.
    Depth = 51_177,
    /// Semantic mask samples.
    PhotometricMask = 52_527,
    /// An unsupported or private photometric interpretation.
    #[num_enum(catch_all)]
    Unknown(u16),
}
