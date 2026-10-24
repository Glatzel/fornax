//! Shared exact metadata values.

use std::num::NonZeroU32;

/// A finite rational metadata value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Rational {
    numerator: i64,
    denominator: NonZeroU32,
}

impl Rational {
    /// Creates a rational value with a non-zero denominator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::Rational;
    ///
    /// let value = Rational::new(1, NonZeroU32::new(2).unwrap());
    /// assert_eq!(value.as_f64(), 0.5);
    /// ```
    pub const fn new(numerator: i64, denominator: NonZeroU32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Returns the numerator.
    pub const fn numerator(self) -> i64 { self.numerator }

    /// Returns the non-zero denominator.
    pub const fn denominator(self) -> NonZeroU32 { self.denominator }

    /// Converts the exact components to `f64` for scalar preprocessing.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::Rational;
    ///
    /// assert_eq!(Rational::new(3, NonZeroU32::new(4).unwrap()).as_f64(), 0.75);
    /// ```
    pub fn as_f64(self) -> f64 { self.numerator as f64 / f64::from(self.denominator.get()) }

    pub(super) const fn is_nonnegative(self) -> bool { self.numerator >= 0 }
}
