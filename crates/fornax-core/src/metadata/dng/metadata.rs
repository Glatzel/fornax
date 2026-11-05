use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use image::math::Rect;

use crate::error::DecodeError;
use crate::metadata::dng::{DngTag, DngValue};

/// Deterministically ordered DNG metadata values keyed by TIFF/DNG tag.
pub struct DngMetadata(BTreeMap<DngTag, DngValue>);
impl Deref for DngMetadata {
    type Target = BTreeMap<DngTag, DngValue>;

    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for DngMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
impl DngMetadata {
    pub fn active_area(&self) -> Result<Rect, DecodeError> {
        let values = self
            .unsigned_values(self.get(&DngTag::ActiveArea))?
            .ok_or_else(|| DecodeError::Backend("ActiveArea is missing".into()))?;
        let [top, left, bottom, right]: [u32; 4] = values
            .try_into()
            .map_err(|_| DecodeError::Backend("ActiveArea must contain four values".into()))?;
        if top >= bottom || left >= right || bottom > full_height || right > full_width {
            return Err(DecodeError::Backend(format!(
                "ActiveArea ({top}, {left}, {bottom}, {right}) is outside {full_width}x{full_height}"
            ).into()));
        }
        Ok(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
    fn unsigned_values(&self, value: Option<&DngValue>) -> Result<Option<Vec<u32>>, DecodeError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let values = match value {
            DngValue::List(values) => values
                .iter()
                .map(|v| self.unsigned_scalar(v))
                .collect::<Result<Vec<_>, _>>()?,
            value => vec![self.unsigned_scalar(value)?],
        };
        Ok(Some(values))
    }
    fn unsigned_scalar(&self, value: &DngValue) -> Result<u32, DecodeError> {
        let number = match value {
            DngValue::Byte(value) => *value as u32,
            DngValue::Short(value) => *value as u32,
            DngValue::Unsigned(value) | DngValue::Ifd(value) => *value,
            DngValue::UnsignedBig(value) => u32::try_from(*value)
                .map_err(|_| DecodeError::Backend("tag value does not fit u32".into()))?,
            DngValue::IfdBig(value) => u32::try_from(*value)
                .map_err(|_| DecodeError::Backend("IFD pointer does not fit u32".into()))?,
            DngValue::Rational(numerator, denominator) if *denominator == 1 => *numerator,
            _ => {
                return Err(DecodeError::Backend(
                    "tag does not contain unsigned integers".into(),
                ));
            }
        };
        Ok(number)
    }
}
