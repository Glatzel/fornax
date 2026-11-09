use std::fmt;

/// A TIFF tag identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tag(pub u16);

impl From<u16> for Tag {
    fn from(value: u16) -> Self { Self(value) }
}

impl From<Tag> for u16 {
    fn from(value: Tag) -> Self { value.0 }
}

impl fmt::Display for Tag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TIFF tag 0x{:04x}", self.0)
    }
}

/// A typed TIFF field value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Byte(u8),
    SignedByte(i8),
    Short(u16),
    SignedShort(i16),
    Unsigned(u32),
    Signed(i32),
    UnsignedBig(u64),
    SignedBig(i64),
    Float(f32),
    Double(f64),
    Rational(u32, u32),
    SRational(i32, i32),
    RationalBig(i64, i64),
    SRationalBig(i64, i64),
    Ifd(u32),
    IfdBig(u64),
    Ascii(String),
    List(Vec<Self>),
}

impl Value {
    /// Returns the number of scalar values represented by this field.
    pub const fn count(&self) -> usize {
        match self {
            Self::List(values) => values.len(),
            _ => 1,
        }
    }

    /// Returns an unsigned integer at `index` when representable.
    pub fn force_u32(&self, index: usize) -> u32 {
        self.scalar(index)
            .and_then(Self::force_u32_scalar)
            .unwrap_or_default()
    }

    /// Returns a floating-point value at `index` when representable.
    pub fn force_f32(&self, index: usize) -> f32 {
        self.scalar(index)
            .and_then(Self::force_f32_scalar)
            .unwrap_or_default()
    }

    /// Converts every scalar to `u64`.
    pub fn into_u64_vec(self) -> Result<Vec<u64>, ValueConversionError> {
        match self {
            Self::List(values) => values.into_iter().map(Self::force_u64_scalar).collect(),
            value => Ok(vec![Self::force_u64_scalar(value)?]),
        }
    }

    fn scalar(&self, index: usize) -> Option<&Self> {
        match self {
            Self::List(values) => values.get(index),
            _ if index == 0 => Some(self),
            _ => None,
        }
    }

    fn force_u32_scalar(&self) -> Option<u32> {
        Some(match self {
            Self::Byte(value) => u32::from(*value),
            Self::Short(value) => u32::from(*value),
            Self::Unsigned(value) | Self::Ifd(value) | Self::Rational(value, 1) => *value,
            Self::UnsignedBig(value) | Self::IfdBig(value) => u32::try_from(*value).ok()?,
            Self::SignedByte(value) => u32::try_from(*value).ok()?,
            Self::SignedShort(value) => u32::try_from(*value).ok()?,
            Self::Signed(value) | Self::SRational(value, 1) => u32::try_from(*value).ok()?,
            Self::SignedBig(value) => u32::try_from(*value).ok()?,
            Self::Float(value) => *value as u32,
            Self::Double(value) => *value as u32,
            _ => return None,
        })
    }

    fn force_u64_scalar(self) -> Result<u64, ValueConversionError> {
        match self {
            Self::Byte(value) => Ok(u64::from(value)),
            Self::Short(value) => Ok(u64::from(value)),
            Self::Unsigned(value) | Self::Ifd(value) => Ok(u64::from(value)),
            Self::UnsignedBig(value) | Self::IfdBig(value) => Ok(value),
            Self::SignedByte(value) => u64::try_from(value).map_err(|_| ValueConversionError),
            Self::SignedShort(value) => u64::try_from(value).map_err(|_| ValueConversionError),
            Self::Signed(value) => u64::try_from(value).map_err(|_| ValueConversionError),
            Self::SignedBig(value) => u64::try_from(value).map_err(|_| ValueConversionError),
            _ => Err(ValueConversionError),
        }
    }

    fn force_f32_scalar(&self) -> Option<f32> {
        Some(match self {
            Self::Byte(value) => f32::from(*value),
            Self::SignedByte(value) => f32::from(*value),
            Self::Short(value) => f32::from(*value),
            Self::SignedShort(value) => f32::from(*value),
            Self::Unsigned(value) | Self::Ifd(value) => *value as f32,
            Self::Signed(value) => *value as f32,
            Self::UnsignedBig(value) | Self::IfdBig(value) => *value as f32,
            Self::SignedBig(value) => *value as f32,
            Self::Float(value) => *value,
            Self::Double(value) => *value as f32,
            Self::Rational(numerator, denominator) => *numerator as f32 / *denominator as f32,
            Self::SRational(numerator, denominator) => *numerator as f32 / *denominator as f32,
            Self::RationalBig(numerator, denominator)
            | Self::SRationalBig(numerator, denominator) => *numerator as f32 / *denominator as f32,
            _ => return None,
        })
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Returns the first unsigned integer represented by a TIFF value.
///
/// This accepts [`Value::Byte`], [`Value::Short`], [`Value::Unsigned`],
/// [`Value::Ifd`], [`Value::UnsignedBig`], and [`Value::IfdBig`]. Nested
/// [`Value::List`] values are followed recursively at index zero. Signed,
/// floating-point, rational, ASCII, and empty values return `None`.
///
/// # Examples
///
/// ```
/// use fornax_tiff::{Value, unsigned_scalar};
///
/// let value = Value::List(vec![Value::Short(7), Value::Short(9)]);
/// assert_eq!(unsigned_scalar(Some(&value)), Some(7));
/// ```
pub fn unsigned_scalar(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Byte(value) => Some(u64::from(*value)),
        Value::Short(value) => Some(u64::from(*value)),
        Value::Unsigned(value) | Value::Ifd(value) => Some(u64::from(*value)),
        Value::UnsignedBig(value) | Value::IfdBig(value) => Some(*value),
        Value::List(values) => unsigned_scalar(values.first()),
        _ => None,
    }
}

/// Converts a TIFF directory field into unsigned integers.
///
/// A scalar becomes a one-element vector and a flat list is converted in order.
/// List elements use [`Value::into_u64_vec`] semantics, so non-negative signed
/// integers are accepted while negative, rational, floating-point, nested-list,
/// and other non-convertible values are rejected.
///
/// # Errors
///
/// Returns [`ValueAccessError::Missing`] when `tag` is absent,
/// [`ValueAccessError::InvalidType`] when a value cannot be converted to an
/// unsigned integer, or [`ValueAccessError::Overflow`] when a converted value
/// does not fit `T`.
///
/// # Examples
///
/// ```
/// use fornax_tiff::{Directory, Tag, Value, unsigned_vec};
///
/// let mut directory = Directory::default();
/// directory.insert(
///     Tag(1),
///     Value::List(vec![Value::Byte(2), Value::Unsigned(3)]),
/// );
/// assert_eq!(
///     unsigned_vec::<_, usize>(&directory, Tag(1)).unwrap(),
///     vec![2, 3],
/// );
/// ```
pub fn unsigned_vec<T, U>(directory: &crate::Directory, tag: T) -> Result<Vec<U>, ValueAccessError>
where
    T: Into<Tag>,
    U: TryFrom<u64>,
{
    let tag = tag.into();
    let value = directory.get(&tag).ok_or(ValueAccessError::Missing)?;
    let values = value
        .clone()
        .into_u64_vec()
        .map_err(|_| ValueAccessError::InvalidType)?;
    values
        .into_iter()
        .map(|value| U::try_from(value).map_err(|_| ValueAccessError::Overflow))
        .collect()
}

/// Failure to convert a TIFF scalar into an unsigned integer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueConversionError;

impl fmt::Display for ValueConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TIFF value is not representable as an unsigned integer")
    }
}

impl std::error::Error for ValueConversionError {}

/// Failure to access unsigned integers from a TIFF directory field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueAccessError {
    /// The requested tag is absent.
    Missing,
    /// The tag value cannot be converted to unsigned integers.
    InvalidType,
    /// A converted value does not fit the requested target integer type.
    Overflow,
}

impl fmt::Display for ValueAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "TIFF tag is missing",
            Self::InvalidType => "TIFF value is not an unsigned integer list",
            Self::Overflow => "TIFF value does not fit the target integer type",
        })
    }
}

impl std::error::Error for ValueAccessError {}
