use std::collections::BTreeMap;
use std::fmt;

/// Unsigned TIFF rational value.
///
/// # Examples
///
/// ```
/// use fornax_decoder::Rational;
///
/// let ratio = Rational::new(1, 2);
/// assert_eq!(ratio.denominator, 2);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rational {
    /// Numerator.
    pub numerator: u32,
    /// Denominator.
    pub denominator: u32,
}

impl Rational {
    /// Creates an unsigned rational value.
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// Signed TIFF rational value.
///
/// # Examples
///
/// ```
/// use fornax_decoder::SignedRational;
///
/// let ratio = SignedRational::new(-1, 2);
/// assert_eq!(ratio.numerator, -1);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedRational {
    /// Numerator.
    pub numerator: i32,
    /// Denominator.
    pub denominator: i32,
}

impl SignedRational {
    /// Creates a signed rational value.
    pub const fn new(numerator: i32, denominator: i32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// DNG tag identifier.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngTag;
///
/// assert_eq!(DngTag::BLACK_LEVEL.value(), 50_714);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DngTag(u16);

impl DngTag {
    /// TIFF `Make` tag.
    pub const MAKE: Self = Self(271);
    /// TIFF `Model` tag.
    pub const MODEL: Self = Self(272);
    /// TIFF `Orientation` tag.
    pub const ORIENTATION: Self = Self(274);
    /// DNG `CFARepeatPatternDim` tag.
    pub const CFA_REPEAT_PATTERN_DIM: Self = Self(33_421);
    /// DNG `CFAPattern` tag.
    pub const CFA_PATTERN: Self = Self(33_422);
    /// DNG `DNGVersion` tag.
    pub const DNG_VERSION: Self = Self(50_706);
    /// DNG `DNGBackwardVersion` tag.
    pub const DNG_BACKWARD_VERSION: Self = Self(50_707);
    /// DNG `UniqueCameraModel` tag.
    pub const UNIQUE_CAMERA_MODEL: Self = Self(50_708);
    /// DNG `CFAPlaneColor` tag.
    pub const CFA_PLANE_COLOR: Self = Self(50_710);
    /// DNG `CFALayout` tag.
    pub const CFA_LAYOUT: Self = Self(50_711);
    /// DNG `LinearizationTable` tag.
    pub const LINEARIZATION_TABLE: Self = Self(50_712);
    /// DNG `BlackLevelRepeatDim` tag.
    pub const BLACK_LEVEL_REPEAT_DIM: Self = Self(50_713);
    /// DNG `BlackLevel` tag.
    pub const BLACK_LEVEL: Self = Self(50_714);
    /// DNG `BlackLevelDeltaH` tag.
    pub const BLACK_LEVEL_DELTA_H: Self = Self(50_715);
    /// DNG `BlackLevelDeltaV` tag.
    pub const BLACK_LEVEL_DELTA_V: Self = Self(50_716);
    /// DNG `WhiteLevel` tag.
    pub const WHITE_LEVEL: Self = Self(50_717);
    /// DNG `DefaultScale` tag.
    pub const DEFAULT_SCALE: Self = Self(50_718);
    /// DNG `DefaultCropOrigin` tag.
    pub const DEFAULT_CROP_ORIGIN: Self = Self(50_719);
    /// DNG `DefaultCropSize` tag.
    pub const DEFAULT_CROP_SIZE: Self = Self(50_720);
    /// DNG `ColorMatrix1` tag.
    pub const COLOR_MATRIX1: Self = Self(50_721);
    /// DNG `ColorMatrix2` tag.
    pub const COLOR_MATRIX2: Self = Self(50_722);
    /// DNG `CameraCalibration1` tag.
    pub const CAMERA_CALIBRATION1: Self = Self(50_723);
    /// DNG `CameraCalibration2` tag.
    pub const CAMERA_CALIBRATION2: Self = Self(50_724);
    /// DNG `AnalogBalance` tag.
    pub const ANALOG_BALANCE: Self = Self(50_727);
    /// DNG `AsShotNeutral` tag.
    pub const AS_SHOT_NEUTRAL: Self = Self(50_728);
    /// DNG `AsShotWhiteXY` tag.
    pub const AS_SHOT_WHITE_XY: Self = Self(50_729);
    /// DNG `BaselineExposure` tag.
    pub const BASELINE_EXPOSURE: Self = Self(50_730);
    /// DNG `BaselineNoise` tag.
    pub const BASELINE_NOISE: Self = Self(50_731);
    /// DNG `BaselineSharpness` tag.
    pub const BASELINE_SHARPNESS: Self = Self(50_732);
    /// DNG `BayerGreenSplit` tag.
    pub const BAYER_GREEN_SPLIT: Self = Self(50_733);
    /// DNG `CameraSerialNumber` tag.
    pub const CAMERA_SERIAL_NUMBER: Self = Self(50_735);
    /// DNG `CalibrationIlluminant1` tag.
    pub const CALIBRATION_ILLUMINANT1: Self = Self(50_778);
    /// DNG `CalibrationIlluminant2` tag.
    pub const CALIBRATION_ILLUMINANT2: Self = Self(50_779);
    /// DNG `ActiveArea` tag.
    pub const ACTIVE_AREA: Self = Self(50_829);
    /// DNG `MaskedAreas` tag.
    pub const MASKED_AREAS: Self = Self(50_830);

    /// Creates a tag from its TIFF tag number.
    pub const fn new(value: u16) -> Self { Self(value) }

    /// Returns the TIFF tag number.
    pub const fn value(self) -> u16 { self.0 }
}

impl fmt::Display for DngTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DNG tag 0x{:04x}", self.0)
    }
}

/// TIFF/DNG field value with its original value type.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngValue;
///
/// let value = DngValue::Short(vec![512, 513]);
/// assert_eq!(value, DngValue::Short(vec![512, 513]));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum DngValue {
    /// `BYTE` values.
    Byte(Vec<u8>),
    /// `ASCII` value without its NUL terminator.
    Ascii(String),
    /// `SHORT` values.
    Short(Vec<u16>),
    /// `LONG` values.
    Long(Vec<u32>),
    /// `RATIONAL` values.
    Rational(Vec<Rational>),
    /// `SBYTE` values.
    SByte(Vec<i8>),
    /// `UNDEFINED` bytes.
    Undefined(Vec<u8>),
    /// `SSHORT` values.
    SShort(Vec<i16>),
    /// `SLONG` values.
    SLong(Vec<i32>),
    /// `SRATIONAL` values.
    SignedRational(Vec<SignedRational>),
    /// `FLOAT` values.
    Float(Vec<f32>),
    /// `DOUBLE` values.
    Double(Vec<f64>),
    /// `IFD` offsets.
    Ifd(Vec<u32>),
    /// BigTIFF `LONG8` values.
    Long8(Vec<u64>),
    /// BigTIFF `SLONG8` values.
    SLong8(Vec<i64>),
    /// BigTIFF `IFD8` offsets.
    Ifd8(Vec<u64>),
}

/// Deterministically ordered DNG tag container.
///
/// # Examples
///
/// ```
/// use fornax_decoder::{DngMetadata, DngTag, DngValue};
///
/// let mut metadata = DngMetadata::new();
/// metadata.insert(DngTag::WHITE_LEVEL, DngValue::Long(vec![16_383]));
/// assert_eq!(metadata.len(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DngMetadata {
    tags: BTreeMap<DngTag, DngValue>,
}

impl DngMetadata {
    /// Creates an empty metadata container.
    pub const fn new() -> Self {
        Self {
            tags: BTreeMap::new(),
        }
    }

    /// Inserts a typed value and returns the previous value for the tag.
    pub fn insert(&mut self, tag: DngTag, value: DngValue) -> Option<DngValue> {
        self.tags.insert(tag, value)
    }

    /// Returns the typed value for a tag.
    pub fn get(&self, tag: DngTag) -> Option<&DngValue> { self.tags.get(&tag) }

    /// Returns `BlackLevel`.
    pub fn black_level(&self) -> Option<&DngValue> { self.get(DngTag::BLACK_LEVEL) }

    /// Returns `CFAPattern`.
    pub fn cfa_pattern(&self) -> Option<&DngValue> { self.get(DngTag::CFA_PATTERN) }

    /// Returns `CFAPlaneColor`.
    pub fn cfa_plane_colors(&self) -> Option<&DngValue> { self.get(DngTag::CFA_PLANE_COLOR) }

    /// Returns the number of stored tags.
    pub fn len(&self) -> usize { self.tags.len() }

    /// Returns `true` when no tags are stored.
    pub fn is_empty(&self) -> bool { self.tags.is_empty() }
}
