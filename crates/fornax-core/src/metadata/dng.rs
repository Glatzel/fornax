use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

use num_enum::{FromPrimitive, IntoPrimitive};
use strum::AsRefStr;
use tiff::tags::Tag;

pub type DngValue = tiff::decoder::ifd::Value;
/// Deterministically ordered DNG metadata values keyed by TIFF/DNG tag.
#[derive(Debug, Default, Clone)]
pub struct DngMetadata(pub BTreeMap<DngTag, DngValue>);
impl Deref for DngMetadata {
    type Target = BTreeMap<DngTag, DngValue>;

    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for DngMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

/// TIFF or DNG tag identifier.
///
/// # Examples
///
/// ```
/// use fornax_core::metadata::dng::DngTag;
///
/// assert_eq!(DngTag::from(274), DngTag::Orientation);
/// assert_eq!(u16::from(DngTag::Orientation), 274);
/// ```
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, AsRefStr, FromPrimitive, IntoPrimitive)]
pub enum DngTag {
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
    /// `SamplesPerPixel`.
    SamplesPerPixel = 277,
    /// `RowsPerStrip`.
    RowsPerStrip = 278,
    /// `StripOffsets`.
    StripOffsets = 273,
    /// `StripByteCounts`.
    StripByteCounts = 279,
    /// `PlanarConfiguration`.
    PlanarConfiguration = 284,
    /// `Make`.
    Make = 271,
    /// `Model`.
    Model = 272,
    /// `Orientation`.
    Orientation = 274,
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
    /// `CFARepeatPatternDim`.
    CfaRepeatPatternDim = 33_421,
    /// `CFAPattern`.
    CfaPattern = 33_422,
    /// `DNGVersion`.
    DngVersion = 50_706,
    /// `DNGBackwardVersion`.
    DngBackwardVersion = 50_707,
    /// `UniqueCameraModel`.
    UniqueCameraModel = 50_708,
    /// `CFAPlaneColor`.
    CfaPlaneColor = 50_710,
    /// `CFALayout`.
    CfaLayout = 50_711,
    /// `LinearizationTable`.
    LinearizationTable = 50_712,
    /// `BlackLevelRepeatDim`.
    BlackLevelRepeatDim = 50_713,
    /// `BlackLevel`.
    BlackLevel = 50_714,
    /// `BlackLevelDeltaH`.
    BlackLevelDeltaH = 50_715,
    /// `BlackLevelDeltaV`.
    BlackLevelDeltaV = 50_716,
    /// `WhiteLevel`.
    WhiteLevel = 50_717,
    /// `DefaultScale`.
    DefaultScale = 50_718,
    /// `DefaultCropOrigin`.
    DefaultCropOrigin = 50_719,
    /// `DefaultCropSize`.
    DefaultCropSize = 50_720,
    /// `ColorMatrix1`.
    ColorMatrix1 = 50_721,
    /// `ColorMatrix2`.
    ColorMatrix2 = 50_722,
    /// `CameraCalibration1`.
    CameraCalibration1 = 50_723,
    /// `CameraCalibration2`.
    CameraCalibration2 = 50_724,
    /// `AnalogBalance`.
    AnalogBalance = 50_727,
    /// `AsShotNeutral`.
    AsShotNeutral = 50_728,
    /// `AsShotWhiteXY`.
    AsShotWhiteXY = 50_729,
    /// `BaselineExposure`.
    BaselineExposure = 50_730,
    /// `BaselineNoise`.
    BaselineNoise = 50_731,
    /// `BaselineSharpness`.
    BaselineSharpness = 50_732,
    /// `BayerGreenSplit`.
    BayerGreenSplit = 50_733,
    /// `CameraSerialNumber`.
    CameraSerialNumber = 50_735,
    /// `CalibrationIlluminant1`.
    CalibrationIlluminant1 = 50_778,
    /// `CalibrationIlluminant2`.
    CalibrationIlluminant2 = 50_779,
    /// `ActiveArea`.
    ActiveArea = 50_829,
    /// `MaskedAreas`.
    MaskedAreas = 50_830,
    /// A private or extension tag.
    #[num_enum(catch_all)]
    Unknown(u16),
}

impl Ord for DngTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { u16::from(*self).cmp(&u16::from(*other)) }
}

impl PartialOrd for DngTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl fmt::Display for DngTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "DNG tag 0x{value:04x}"),
            _ => write!(formatter, "{self:?}"),
        }
    }
}

impl From<Tag> for DngTag {
    fn from(value: Tag) -> Self { Self::from(value.to_u16()) }
}

impl From<DngTag> for Tag {
    fn from(value: DngTag) -> Self {
        match value {
            DngTag::NewSubfileType => Self::NewSubfileType,
            DngTag::ImageWidth => Self::ImageWidth,
            DngTag::ImageLength => Self::ImageLength,
            DngTag::BitsPerSample => Self::BitsPerSample,
            DngTag::Compression => Self::Compression,
            DngTag::PhotometricInterpretation => Self::PhotometricInterpretation,
            DngTag::SamplesPerPixel => Self::SamplesPerPixel,
            DngTag::RowsPerStrip => Self::RowsPerStrip,
            DngTag::StripOffsets => Self::StripOffsets,
            DngTag::StripByteCounts => Self::StripByteCounts,
            DngTag::PlanarConfiguration => Self::PlanarConfiguration,
            DngTag::Make => Self::Make,
            DngTag::Model => Self::Model,
            DngTag::Orientation => Self::Orientation,
            DngTag::TileWidth => Self::TileWidth,
            DngTag::TileLength => Self::TileLength,
            DngTag::TileOffsets => Self::TileOffsets,
            DngTag::TileByteCounts => Self::TileByteCounts,
            DngTag::SubIfds => Self::SubIfd,
            tag => Self::Unknown(tag.into()),
        }
    }
}
