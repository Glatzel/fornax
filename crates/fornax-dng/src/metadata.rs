use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use num_enum::{FromPrimitive, IntoPrimitive};
use strum::AsRefStr;

pub type DngValue = fornax_tiff::Value;
/// Deterministically ordered DNG metadata values keyed by TIFF/DNG tag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DngMetadata(Arc<BTreeMap<DngTag, DngValue>>);

impl DngMetadata {
    pub fn insert(&mut self, tag: DngTag, value: DngValue) -> Option<DngValue> {
        Arc::make_mut(&mut self.0).insert(tag, value)
    }

    pub fn remove(&mut self, tag: &DngTag) -> Option<DngValue> {
        Arc::make_mut(&mut self.0).remove(tag)
    }

    pub fn extend<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (DngTag, DngValue)>,
    {
        Arc::make_mut(&mut self.0).extend(values);
    }

    #[cfg(feature = "metadata-json")]
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::{Map, Value as JsonValue};

        let tags = self
            .iter()
            .map(|(tag, value)| {
                let mut object = Map::new();
                object.insert("tag".into(), JsonValue::from(u16::from(*tag)));
                object.insert(
                    "name".into(),
                    JsonValue::String(match tag {
                        DngTag::Unknown(value) => format!("Unknown(0x{value:04x})"),
                        _ => tag.as_ref().to_owned(),
                    }),
                );
                object.insert("value".into(), value_to_json(value));
                JsonValue::Object(object)
            })
            .collect();
        let mut root = Map::new();
        root.insert("tags".into(), JsonValue::Array(tags));
        JsonValue::Object(root)
    }
}

impl Deref for DngMetadata {
    type Target = BTreeMap<DngTag, DngValue>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

/// TIFF or DNG tag identifier.
///
/// # Examples
///
/// ```
/// use fornax_dng::DngTag;
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
fn value_to_json(value: &DngValue) -> serde_json::Value {
    use serde_json::Value as JsonValue;

    match value {
        DngValue::Byte(value) => JsonValue::from(*value),
        DngValue::SignedByte(value) => JsonValue::from(*value),
        DngValue::Short(value) => JsonValue::from(*value),
        DngValue::SignedShort(value) => JsonValue::from(*value),
        DngValue::Unsigned(value) | DngValue::Ifd(value) => JsonValue::from(*value),
        DngValue::Signed(value) => JsonValue::from(*value),
        DngValue::UnsignedBig(value) | DngValue::IfdBig(value) => JsonValue::from(*value),
        DngValue::SignedBig(value) => JsonValue::from(*value),
        DngValue::Float(value) => float_to_json(f64::from(*value)),
        DngValue::Double(value) => float_to_json(*value),
        DngValue::Rational(numerator, denominator) => {
            rational_to_json(i64::from(*numerator), i64::from(*denominator))
        }
        DngValue::SRational(numerator, denominator) => {
            rational_to_json(i64::from(*numerator), i64::from(*denominator))
        }
        DngValue::RationalBig(numerator, denominator)
        | DngValue::SRationalBig(numerator, denominator) => {
            rational_to_json(*numerator, *denominator)
        }
        DngValue::Ascii(value) => JsonValue::String(value.clone()),
        DngValue::List(values) => JsonValue::Array(values.iter().map(value_to_json).collect()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn cloning_metadata_shares_the_underlying_map() {
        let mut metadata = DngMetadata::default();
        metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(4));

        let clone = metadata.clone();

        assert!(Arc::ptr_eq(&metadata.0, &clone.0));
    }

    #[cfg(feature = "metadata-json")]
    #[test]
    fn metadata_json_preserves_tag_types_and_order() {
        let mut metadata = DngMetadata::default();
        metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(4));
        metadata.insert(DngTag::Make, DngValue::Ascii("Fornax".into()));
        metadata.insert(
            DngTag::ColorMatrix1,
            DngValue::List(vec![DngValue::SRational(-1, 2), DngValue::SRational(1, 3)]),
        );
        metadata.insert(
            DngTag::Unknown(60_001),
            DngValue::List(vec![DngValue::Byte(1), DngValue::Byte(2)]),
        );

        let json = metadata.to_json_value();
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
