use std::fmt;

use fornax_core::BayerPattern;
use thiserror::Error;

/// Repeating color-filter-array descriptor stored in row-major order.
///
/// Each pattern entry is a plane index into the DNG `CFAPlaneColor` mapping.
/// With the standard RGB mapping, plane indices `0`, `1`, and `2` denote red,
/// green, and blue respectively. Arbitrary plane counts remain valid for
/// decoding and DNG export but cannot cross the Bayer preprocessor handoff.
///
/// # Examples
///
/// ```
/// use fornax_decoder::CfaPattern;
///
/// let cfa = CfaPattern::new(2, 2, vec![0, 1, 1, 2])?;
/// assert_eq!(cfa.pattern(), [0, 1, 1, 2]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CfaPattern {
    width: u16,
    height: u16,
    pattern: Box<[u8]>,
}

impl CfaPattern {
    /// Creates a CFA descriptor from its repeat width, repeat height, and
    /// plane indices.
    ///
    /// # Errors
    ///
    /// Returns [`CfaPatternError`] when a dimension is zero or the pattern
    /// length does not equal their product.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_decoder::{CfaPattern, CfaPatternError};
    ///
    /// let error = CfaPattern::new(2, 2, vec![0, 1, 1]).unwrap_err();
    /// assert!(matches!(error, CfaPatternError::LengthMismatch { .. }));
    /// ```
    pub fn new(
        width: u16,
        height: u16,
        pattern: impl Into<Box<[u8]>>,
    ) -> Result<Self, CfaPatternError> {
        if width == 0 || height == 0 {
            return Err(CfaPatternError::Empty);
        }
        let expected = usize::from(width)
            .checked_mul(usize::from(height))
            .ok_or(CfaPatternError::DimensionsOverflow)?;
        let pattern = pattern.into();
        if pattern.len() != expected {
            return Err(CfaPatternError::LengthMismatch {
                expected,
                actual: pattern.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pattern,
        })
    }

    /// Returns the horizontal repeat length in pixels.
    pub const fn width(&self) -> u16 { self.width }

    /// Returns the vertical repeat length in pixels.
    pub const fn height(&self) -> u16 { self.height }

    /// Returns the plane indices in row-major order.
    pub fn pattern(&self) -> &[u8] { &self.pattern }

    /// Converts a conventional 2x2 Bayer shape to Fornax's Bayer pattern.
    ///
    /// The caller must separately validate that `CFAPlaneColor` is the
    /// standard RGB mapping. [`crate::DecodedRaw::preprocessor_pattern`]
    /// performs both checks. This method returns `None` for X-Trans, larger,
    /// or otherwise non-2x2 layouts.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::BayerPattern;
    /// use fornax_decoder::CfaPattern;
    ///
    /// let cfa = CfaPattern::new(2, 2, vec![2, 1, 1, 0])?;
    /// assert_eq!(cfa.bayer_pattern(), Some(BayerPattern::BGGR));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn bayer_pattern(&self) -> Option<BayerPattern> {
        if self.width != 2 || self.height != 2 {
            return None;
        }
        match self.pattern.as_ref() {
            [0, 1, 1, 2] => Some(BayerPattern::RGGB),
            [2, 1, 1, 0] => Some(BayerPattern::BGGR),
            [1, 0, 2, 1] => Some(BayerPattern::GRBG),
            [1, 2, 0, 1] => Some(BayerPattern::GBRG),
            _ => None,
        }
    }
}

impl fmt::Display for CfaPattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}x{} {:?}",
            self.width, self.height, self.pattern
        )
    }
}

/// Invalid CFA descriptor.
///
/// # Examples
///
/// ```
/// use fornax_decoder::CfaPatternError;
///
/// let error = CfaPatternError::Empty;
/// assert_eq!(error.to_string(), "CFA repeat dimensions must not be zero");
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CfaPatternError {
    /// One or both repeat dimensions are zero.
    #[error("CFA repeat dimensions must not be zero")]
    Empty,
    /// The product of the repeat dimensions exceeds `usize`.
    #[error("CFA repeat dimensions exceed the platform address space")]
    DimensionsOverflow,
    /// The number of plane entries does not match the repeat dimensions.
    #[error("CFA pattern has {actual} entries, expected {expected}")]
    LengthMismatch {
        /// Required number of entries.
        expected: usize,
        /// Supplied number of entries.
        actual: usize,
    },
}
