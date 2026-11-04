use std::collections::BTreeMap;
use std::fmt;

use strum::AsRefStr;
pub use tiff::decoder::ifd::Value as DngValue;
use tiff::tags::Tag;

/// Deterministically ordered DNG metadata values keyed by TIFF/DNG tag.
pub type DngMetadata = BTreeMap<DngTag, DngValue>;

/// TIFF or DNG tag identifier.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngTag;
///
/// assert_eq!(DngTag::from(274), DngTag::Orientation);
/// assert_eq!(u16::from(DngTag::Orientation), 274);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, AsRefStr)]
pub enum DngTag {
    /// `ImageWidth`.
    ImageWidth,
    /// `ImageLength`.
    ImageLength,
    /// `BitsPerSample`.
    BitsPerSample,
    /// `Compression`.
    Compression,
    /// `PhotometricInterpretation`.
    PhotometricInterpretation,
    /// `SamplesPerPixel`.
    SamplesPerPixel,
    /// `RowsPerStrip`.
    RowsPerStrip,
    /// `StripOffsets`.
    StripOffsets,
    /// `StripByteCounts`.
    StripByteCounts,
    /// `PlanarConfiguration`.
    PlanarConfiguration,
    /// `Make`.
    Make,
    /// `Model`.
    Model,
    /// `Orientation`.
    Orientation,
    /// `CFARepeatPatternDim`.
    CfaRepeatPatternDim,
    /// `CFAPattern`.
    CfaPattern,
    /// `DNGVersion`.
    DngVersion,
    /// `DNGBackwardVersion`.
    DngBackwardVersion,
    /// `UniqueCameraModel`.
    UniqueCameraModel,
    /// `CFAPlaneColor`.
    CfaPlaneColor,
    /// `CFALayout`.
    CfaLayout,
    /// `LinearizationTable`.
    LinearizationTable,
    /// `BlackLevelRepeatDim`.
    BlackLevelRepeatDim,
    /// `BlackLevel`.
    BlackLevel,
    /// `BlackLevelDeltaH`.
    BlackLevelDeltaH,
    /// `BlackLevelDeltaV`.
    BlackLevelDeltaV,
    /// `WhiteLevel`.
    WhiteLevel,
    /// `DefaultScale`.
    DefaultScale,
    /// `DefaultCropOrigin`.
    DefaultCropOrigin,
    /// `DefaultCropSize`.
    DefaultCropSize,
    /// `ColorMatrix1`.
    ColorMatrix1,
    /// `ColorMatrix2`.
    ColorMatrix2,
    /// `CameraCalibration1`.
    CameraCalibration1,
    /// `CameraCalibration2`.
    CameraCalibration2,
    /// `AnalogBalance`.
    AnalogBalance,
    /// `AsShotNeutral`.
    AsShotNeutral,
    /// `AsShotWhiteXY`.
    AsShotWhiteXY,
    /// `BaselineExposure`.
    BaselineExposure,
    /// `BaselineNoise`.
    BaselineNoise,
    /// `BaselineSharpness`.
    BaselineSharpness,
    /// `BayerGreenSplit`.
    BayerGreenSplit,
    /// `CameraSerialNumber`.
    CameraSerialNumber,
    /// `CalibrationIlluminant1`.
    CalibrationIlluminant1,
    /// `CalibrationIlluminant2`.
    CalibrationIlluminant2,
    /// `ActiveArea`.
    ActiveArea,
    /// `MaskedAreas`.
    MaskedAreas,
    /// A private or extension tag.
    Unknown(u16),
}

impl Ord for DngTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.to_u16().cmp(&other.to_u16()) }
}

impl PartialOrd for DngTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl DngTag {
    /// Returns the TIFF tag number.
    pub const fn to_u16(self) -> u16 {
        match self {
            Self::ImageWidth => 256,
            Self::ImageLength => 257,
            Self::BitsPerSample => 258,
            Self::Compression => 259,
            Self::PhotometricInterpretation => 262,
            Self::Make => 271,
            Self::Model => 272,
            Self::StripOffsets => 273,
            Self::Orientation => 274,
            Self::SamplesPerPixel => 277,
            Self::RowsPerStrip => 278,
            Self::StripByteCounts => 279,
            Self::PlanarConfiguration => 284,
            Self::CfaRepeatPatternDim => 33_421,
            Self::CfaPattern => 33_422,
            Self::DngVersion => 50_706,
            Self::DngBackwardVersion => 50_707,
            Self::UniqueCameraModel => 50_708,
            Self::CfaPlaneColor => 50_710,
            Self::CfaLayout => 50_711,
            Self::LinearizationTable => 50_712,
            Self::BlackLevelRepeatDim => 50_713,
            Self::BlackLevel => 50_714,
            Self::BlackLevelDeltaH => 50_715,
            Self::BlackLevelDeltaV => 50_716,
            Self::WhiteLevel => 50_717,
            Self::DefaultScale => 50_718,
            Self::DefaultCropOrigin => 50_719,
            Self::DefaultCropSize => 50_720,
            Self::ColorMatrix1 => 50_721,
            Self::ColorMatrix2 => 50_722,
            Self::CameraCalibration1 => 50_723,
            Self::CameraCalibration2 => 50_724,
            Self::AnalogBalance => 50_727,
            Self::AsShotNeutral => 50_728,
            Self::AsShotWhiteXY => 50_729,
            Self::BaselineExposure => 50_730,
            Self::BaselineNoise => 50_731,
            Self::BaselineSharpness => 50_732,
            Self::BayerGreenSplit => 50_733,
            Self::CameraSerialNumber => 50_735,
            Self::CalibrationIlluminant1 => 50_778,
            Self::CalibrationIlluminant2 => 50_779,
            Self::ActiveArea => 50_829,
            Self::MaskedAreas => 50_830,
            Self::Unknown(value) => value,
        }
    }
}

impl fmt::Display for DngTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "DNG tag 0x{value:04x}"),
            _ => write!(formatter, "{self:?}"),
        }
    }
}

impl From<u16> for DngTag {
    fn from(value: u16) -> Self {
        match value {
            256 => Self::ImageWidth,
            257 => Self::ImageLength,
            258 => Self::BitsPerSample,
            259 => Self::Compression,
            262 => Self::PhotometricInterpretation,
            271 => Self::Make,
            272 => Self::Model,
            273 => Self::StripOffsets,
            274 => Self::Orientation,
            277 => Self::SamplesPerPixel,
            278 => Self::RowsPerStrip,
            279 => Self::StripByteCounts,
            284 => Self::PlanarConfiguration,
            33_421 => Self::CfaRepeatPatternDim,
            33_422 => Self::CfaPattern,
            50_706 => Self::DngVersion,
            50_707 => Self::DngBackwardVersion,
            50_708 => Self::UniqueCameraModel,
            50_710 => Self::CfaPlaneColor,
            50_711 => Self::CfaLayout,
            50_712 => Self::LinearizationTable,
            50_713 => Self::BlackLevelRepeatDim,
            50_714 => Self::BlackLevel,
            50_715 => Self::BlackLevelDeltaH,
            50_716 => Self::BlackLevelDeltaV,
            50_717 => Self::WhiteLevel,
            50_718 => Self::DefaultScale,
            50_719 => Self::DefaultCropOrigin,
            50_720 => Self::DefaultCropSize,
            50_721 => Self::ColorMatrix1,
            50_722 => Self::ColorMatrix2,
            50_723 => Self::CameraCalibration1,
            50_724 => Self::CameraCalibration2,
            50_727 => Self::AnalogBalance,
            50_728 => Self::AsShotNeutral,
            50_729 => Self::AsShotWhiteXY,
            50_730 => Self::BaselineExposure,
            50_731 => Self::BaselineNoise,
            50_732 => Self::BaselineSharpness,
            50_733 => Self::BayerGreenSplit,
            50_735 => Self::CameraSerialNumber,
            50_778 => Self::CalibrationIlluminant1,
            50_779 => Self::CalibrationIlluminant2,
            50_829 => Self::ActiveArea,
            50_830 => Self::MaskedAreas,
            value => Self::Unknown(value),
        }
    }
}

impl From<DngTag> for u16 {
    fn from(value: DngTag) -> Self { value.to_u16() }
}

impl From<Tag> for DngTag {
    fn from(value: Tag) -> Self { Self::from(value.to_u16()) }
}

impl From<DngTag> for Tag {
    fn from(value: DngTag) -> Self {
        match value {
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
            tag => Self::Unknown(tag.to_u16()),
        }
    }
}
