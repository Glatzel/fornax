//! Black and white level metadata.

use std::num::NonZeroU32;

use super::value::Rational;
use crate::error::{MetadataError, RawValidationError};

/// A finite repeating black-level tile in stored-sample units.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BlackTile {
    rows: NonZeroU32,
    columns: NonZeroU32,
    samples_per_plane: NonZeroU32,
    values: Box<[Rational]>,
}

impl BlackTile {
    /// Creates a black-level tile with an exact value count.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] for negative values or
    /// when `values` does not contain `rows * columns * samples_per_plane`
    /// entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::{BlackTile, Rational};
    ///
    /// let one = NonZeroU32::new(1).unwrap();
    /// let black = Rational::new(64, one);
    /// assert!(BlackTile::try_new(one, one, one, vec![black]).is_ok());
    /// ```
    pub fn try_new(
        rows: NonZeroU32,
        columns: NonZeroU32,
        samples_per_plane: NonZeroU32,
        values: Vec<Rational>,
    ) -> Result<Self, MetadataError> {
        let expected = u64::from(rows.get())
            .checked_mul(u64::from(columns.get()))
            .and_then(|count| count.checked_mul(u64::from(samples_per_plane.get())))
            .ok_or(MetadataError::InvalidLevels)?;
        if u64::try_from(values.len()) != Ok(expected)
            || values.iter().any(|value| !value.is_nonnegative())
        {
            return Err(MetadataError::InvalidLevels);
        }
        Ok(Self {
            rows,
            columns,
            samples_per_plane,
            values: values.into_boxed_slice(),
        })
    }

    /// Returns the repeat row count.
    pub const fn rows(&self) -> NonZeroU32 { self.rows }

    /// Returns the repeat column count.
    pub const fn columns(&self) -> NonZeroU32 { self.columns }

    /// Returns the sample-plane count per tile position.
    pub const fn samples_per_plane(&self) -> NonZeroU32 { self.samples_per_plane }

    /// Returns tile values in row-major order.
    pub fn values(&self) -> &[Rational] { &self.values }
}

/// Scalar, four-slot, and repeating black-level metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BlackLevels {
    common: Rational,
    per_slot: [Rational; 4],
    repeat: Option<BlackTile>,
}

impl BlackLevels {
    /// Creates black levels in stored-sample units.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] when any scalar is
    /// negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::{BlackLevels, Rational};
    ///
    /// let black = Rational::new(64, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     BlackLevels::try_new(black, [black; 4], None)
    ///         .unwrap()
    ///         .common(),
    ///     black
    /// );
    /// ```
    pub fn try_new(
        common: Rational,
        per_slot: [Rational; 4],
        repeat: Option<BlackTile>,
    ) -> Result<Self, MetadataError> {
        if !common.is_nonnegative() || per_slot.iter().any(|value| !value.is_nonnegative()) {
            return Err(MetadataError::InvalidLevels);
        }
        Ok(Self {
            common,
            per_slot,
            repeat,
        })
    }

    /// Returns the scalar common component.
    pub const fn common(&self) -> Rational { self.common }

    /// Returns the four CFA-slot components.
    pub const fn per_slot(&self) -> &[Rational; 4] { &self.per_slot }

    /// Returns an optional repeating tile.
    pub const fn repeat(&self) -> Option<&BlackTile> { self.repeat.as_ref() }

    pub(super) fn validate(&self) -> Result<(), MetadataError> {
        if !self.common.is_nonnegative()
            || self.per_slot.iter().any(|value| !value.is_nonnegative())
        {
            Err(MetadataError::InvalidLevels)
        } else {
            Ok(())
        }
    }

    fn repeat_maximum(&self) -> f64 {
        self.repeat.as_ref().map_or(0.0, |tile| {
            tile.values
                .iter()
                .map(|value| value.as_f64())
                .fold(0.0_f64, f64::max)
        })
    }

    fn effective_slot(&self, slot: usize) -> f64 {
        self.common.as_f64() + self.per_slot[slot].as_f64() + self.repeat_maximum()
    }

    fn effective_maximum(&self) -> f64 {
        (0..4)
            .map(|slot| self.effective_slot(slot))
            .fold(0.0_f64, f64::max)
    }
}

/// Declared, decoder-adjusted, and measured white levels.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WhiteLevels {
    declared: Box<[Rational]>,
    adjusted_maximum: Option<Box<[Rational]>>,
    measured_maximum: Option<Box<[Rational]>>,
}

impl WhiteLevels {
    /// Creates white levels with one scalar or four CFA-slot values.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] for unsupported lengths,
    /// non-positive values, or mismatched optional arrays.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::{Rational, WhiteLevels};
    ///
    /// let white = Rational::new(4095, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     WhiteLevels::try_new(vec![white], None, None)
    ///         .unwrap()
    ///         .declared(),
    ///     &[white]
    /// );
    /// ```
    pub fn try_new(
        declared: Vec<Rational>,
        adjusted_maximum: Option<Vec<Rational>>,
        measured_maximum: Option<Vec<Rational>>,
    ) -> Result<Self, MetadataError> {
        let valid_primary =
            matches!(declared.len(), 1 | 4) && declared.iter().all(|value| value.as_f64() > 0.0);
        let positive_adjusted = adjusted_maximum.as_ref().is_none_or(|values| {
            values.len() == declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        let positive_measured = measured_maximum.as_ref().is_none_or(|values| {
            values.len() == declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        if !valid_primary || !positive_adjusted || !positive_measured {
            return Err(MetadataError::InvalidLevels);
        }
        Ok(Self {
            declared: declared.into_boxed_slice(),
            adjusted_maximum: adjusted_maximum.map(Vec::into_boxed_slice),
            measured_maximum: measured_maximum.map(Vec::into_boxed_slice),
        })
    }

    /// Returns source-declared white levels.
    pub fn declared(&self) -> &[Rational] { &self.declared }

    /// Returns decoder-adjusted maximum values.
    pub fn adjusted_maximum(&self) -> Option<&[Rational]> { self.adjusted_maximum.as_deref() }

    /// Returns measured maximum values.
    pub fn measured_maximum(&self) -> Option<&[Rational]> { self.measured_maximum.as_deref() }

    pub(super) fn validate(&self) -> Result<(), MetadataError> {
        let primary_valid = matches!(self.declared.len(), 1 | 4)
            && self.declared.iter().all(|value| value.as_f64() > 0.0);
        let adjusted_valid = self.adjusted_maximum.as_ref().is_none_or(|values| {
            values.len() == self.declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        let measured_valid = self.measured_maximum.as_ref().is_none_or(|values| {
            values.len() == self.declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        if primary_valid && adjusted_valid && measured_valid {
            Ok(())
        } else {
            Err(MetadataError::InvalidLevels)
        }
    }
}

pub(super) fn validate_level_relationship(
    black: &BlackLevels,
    white: &WhiteLevels,
) -> Result<(), RawValidationError> {
    let valid = match white.declared.as_ref() {
        [scalar] => scalar.as_f64() > black.effective_maximum(),
        values if values.len() == 4 => values
            .iter()
            .enumerate()
            .all(|(slot, value)| value.as_f64() > black.effective_slot(slot)),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(RawValidationError::Metadata(MetadataError::InvalidLevels))
    }
}
