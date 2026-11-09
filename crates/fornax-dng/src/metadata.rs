use std::fmt;

use fornax_core::error::DecodeError;
use fornax_core::protocol::IDecoder;
pub use fornax_tiff::constants::TiffTag;
pub use fornax_tiff::{Directory, Tag, Value};
use num_enum::{FromPrimitive, IntoPrimitive};
use strum::AsRefStr;

pub trait IDecoderDng: Send + Sync + IDecoder {
    fn dng_metadata(&self) -> Result<Directory, DecodeError>;
}

/// Converts a TIFF/DNG directory into deterministic JSON metadata.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "metadata-json")]
/// # {
/// use fornax_dng::metadata::{DngTag, Tag, Value, directory_to_json_value};
/// use fornax_tiff::Directory;
///
/// let mut directory = Directory::default();
/// directory.insert(Tag::from(DngTag::DngVersion), Value::Byte(1));
/// let json = directory_to_json_value(&directory);
/// assert_eq!(json["tags"][0]["name"], "DngVersion");
/// # }
/// ```
#[cfg(feature = "metadata-json")]
pub fn directory_to_json_value(directory: &Directory) -> serde_json::Value {
    use serde_json::{Map, Value as JsonValue};

    let tags = directory
        .iter()
        .map(|(tag, value)| {
            let mut object = Map::new();
            object.insert("tag".into(), JsonValue::from(tag.0));
            object.insert("name".into(), JsonValue::String(tag_name(*tag)));
            object.insert("value".into(), value_to_json(value));
            JsonValue::Object(object)
        })
        .collect();
    let mut root = Map::new();
    root.insert("tags".into(), JsonValue::Array(tags));
    JsonValue::Object(root)
}

#[cfg(feature = "metadata-json")]
fn tag_name(tag: Tag) -> String {
    use fornax_tiff::constants::TiffTag;

    let dng_tag = DngTag::from(tag);
    if !matches!(dng_tag, DngTag::Unknown(_)) {
        return dng_tag.as_ref().to_owned();
    }
    let tiff_tag = TiffTag::from(tag);
    match tiff_tag {
        TiffTag::Unknown(value) => format!("Unknown(0x{value:04x})"),
        _ => tiff_tag.as_ref().to_owned(),
    }
}

/// DNG tag identifier.
///
/// # Examples
///
/// ```
/// use fornax_dng::DngTag;
///
/// assert_eq!(DngTag::from(50_706), DngTag::DngVersion);
/// assert_eq!(u16::from(DngTag::DngVersion), 50_706);
/// ```
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, AsRefStr, FromPrimitive, IntoPrimitive)]
pub enum DngTag {
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
    /// `ForwardMatrix1`.
    ForwardMatrix1 = 50_964,
    /// `ForwardMatrix2`.
    ForwardMatrix2 = 50_965,
    /// `CameraCalibration1`.
    CameraCalibration1 = 50_723,
    /// `CameraCalibration2`.
    CameraCalibration2 = 50_724,
    ///`ReductionMatrix1`
    ReductionMatrix1 = 50_725,
    ///`ReductionMatrix2`
    ReductionMatrix2 = 50_726,
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
    /// `DNGPrivateData`.
    DngPrivateData = 50_740,
    /// `CalibrationIlluminant1`.
    CalibrationIlluminant1 = 50_778,
    /// `CalibrationIlluminant2`.
    CalibrationIlluminant2 = 50_779,
    /// `ActiveArea`.
    ActiveArea = 50_829,
    /// `MaskedAreas`.
    MaskedAreas = 50_830,
    RowInterleaveFactor = 50_975,
    OpcodeList1 = 51_008,
    CalibrationIlluminant3 = 52529,
    CameraCalibration3 = 52530,
    ColorMatrix3 = 52531,
    ForwardMatrix3 = 52532,
    IlluminantData1 = 52533,
    IlluminantData2 = 52534,
    IlluminantData3 = 52535,
    ReductionMatrix3 = 52538,
    ColumnInterleaveFactor = 50_976,
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

impl From<fornax_tiff::Tag> for DngTag {
    fn from(value: fornax_tiff::Tag) -> Self { Self::from(value.0) }
}

impl From<DngTag> for fornax_tiff::Tag {
    fn from(value: DngTag) -> Self { Self(u16::from(value)) }
}

#[cfg(feature = "metadata-json")]
fn rational_to_json(numerator: i64, denominator: i64) -> serde_json::Value {
    use serde_json::{Map, Value as JsonValue};

    let mut object = Map::new();
    object.insert("numerator".into(), JsonValue::from(numerator));
    object.insert("denominator".into(), JsonValue::from(denominator));
    JsonValue::Object(object)
}

#[cfg(feature = "metadata-json")]
fn float_to_json(value: f64) -> serde_json::Value {
    use serde_json::{Number, Value as JsonValue};

    Number::from_f64(value).map_or_else(|| JsonValue::String(value.to_string()), JsonValue::Number)
}

#[cfg(feature = "metadata-json")]
fn value_to_json(value: &Value) -> serde_json::Value {
    use serde_json::Value as JsonValue;

    match value {
        Value::Byte(value) => JsonValue::from(*value),
        Value::SignedByte(value) => JsonValue::from(*value),
        Value::Short(value) => JsonValue::from(*value),
        Value::SignedShort(value) => JsonValue::from(*value),
        Value::Unsigned(value) | Value::Ifd(value) => JsonValue::from(*value),
        Value::Signed(value) => JsonValue::from(*value),
        Value::UnsignedBig(value) | Value::IfdBig(value) => JsonValue::from(*value),
        Value::SignedBig(value) => JsonValue::from(*value),
        Value::Float(value) => float_to_json(f64::from(*value)),
        Value::Double(value) => float_to_json(*value),
        Value::Rational(numerator, denominator) => {
            rational_to_json(i64::from(*numerator), i64::from(*denominator))
        }
        Value::SRational(numerator, denominator) => {
            rational_to_json(i64::from(*numerator), i64::from(*denominator))
        }
        Value::RationalBig(numerator, denominator)
        | Value::SRationalBig(numerator, denominator) => rational_to_json(*numerator, *denominator),
        Value::Ascii(value) => JsonValue::String(value.clone()),
        Value::List(values) => JsonValue::Array(values.iter().map(value_to_json).collect()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dng_tags_round_trip_through_numeric_and_dynamic_tags() {
        use super::*;

        assert_eq!(DngTag::from(50_706), DngTag::DngVersion);
        assert_eq!(u16::from(DngTag::DngVersion), 50_706_u16);
        assert_eq!(Tag::from(DngTag::DngVersion), Tag(50_706));
        assert_eq!(DngTag::from(Tag(50_706)), DngTag::DngVersion);
    }

    #[cfg(feature = "metadata-json")]
    #[test]
    fn metadata_json_preserves_tag_types_and_order() {
        use fornax_tiff::constants::TiffTag;

        use super::*;
        let mut metadata = Directory::default();
        metadata.insert(TiffTag::ImageWidth.into(), Value::Unsigned(4));
        metadata.insert(TiffTag::Make.into(), Value::Ascii("Fornax".into()));
        metadata.insert(
            DngTag::ColorMatrix1.into(),
            Value::List(vec![Value::SRational(-1, 2), Value::SRational(1, 3)]),
        );
        metadata.insert(
            DngTag::Unknown(60_001).into(),
            Value::List(vec![Value::Byte(1), Value::Byte(2)]),
        );

        let json = directory_to_json_value(&metadata);
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags[0]["name"], "ImageWidth");
        assert_eq!(tags[0]["value"], 4);
        assert_eq!(tags[1]["name"], "Make");
        assert_eq!(tags[1]["value"], "Fornax");
        assert_eq!(tags[2]["value"][0]["numerator"], -1);
        assert_eq!(tags[2]["value"][0]["denominator"], 2);
        assert_eq!(tags[3]["name"], "Unknown(0xea61)");
        assert_eq!(tags[3]["value"], serde_json::json!([1, 2]));
    }
}
