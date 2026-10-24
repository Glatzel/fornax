//! Pixel geometry, stored dimensions, and ActiveArea coordinates.
//!
//! # Examples
//!
//! ```
//! use fornax::geometry::{ActiveArea, PixelSize};
//!
//! let stored = PixelSize::new(8, 6).unwrap();
//! let active = ActiveArea::try_new(stored, 1, 1, 5, 7).unwrap();
//! assert_eq!(active.rect().width(), 6);
//! ```

use std::num::NonZeroU64;

use crate::error::RawValidationError;

/// A non-empty two-dimensional pixel extent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PixelSize {
    width: u32,
    height: u32,
}

impl PixelSize {
    /// Creates a non-empty pixel size.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::PixelSize;
    ///
    /// assert_eq!(PixelSize::new(4, 3).unwrap().width(), 4);
    /// assert_eq!(PixelSize::new(0, 3), None);
    /// ```
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            None
        } else {
            Some(Self { width, height })
        }
    }

    /// Returns the width in pixels.
    pub const fn width(self) -> u32 { self.width }

    /// Returns the height in pixels.
    pub const fn height(self) -> u32 { self.height }

    pub(crate) fn sample_count(self) -> Result<usize, RawValidationError> {
        let count = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .ok_or(RawValidationError::InvalidGeometry)?;
        usize::try_from(count).map_err(|_| RawValidationError::InvalidGeometry)
    }
}

/// A pixel coordinate in the stored sensor raster.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PixelCoord {
    /// Zero-based row in the stored raster.
    pub row: u32,
    /// Zero-based column in the stored raster.
    pub col: u32,
}

impl PixelCoord {
    /// Creates a stored-raster coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::PixelCoord;
    ///
    /// assert_eq!(PixelCoord::new(2, 5).col, 5);
    /// ```
    pub const fn new(row: u32, col: u32) -> Self { Self { row, col } }
}

/// A non-empty, half-open rectangle in stored pixel coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PixelRect {
    top: u32,
    left: u32,
    bottom: u32,
    right: u32,
}

impl PixelRect {
    /// Creates a non-empty rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::PixelRect;
    ///
    /// let rect = PixelRect::new(1, 2, 4, 6).unwrap();
    /// assert_eq!(rect.width(), 4);
    /// ```
    pub const fn new(top: u32, left: u32, bottom: u32, right: u32) -> Option<Self> {
        if top < bottom && left < right {
            Some(Self {
                top,
                left,
                bottom,
                right,
            })
        } else {
            None
        }
    }

    /// Returns the top inclusive bound.
    pub const fn top(self) -> u32 { self.top }

    /// Returns the left inclusive bound.
    pub const fn left(self) -> u32 { self.left }

    /// Returns the bottom exclusive bound.
    pub const fn bottom(self) -> u32 { self.bottom }

    /// Returns the right exclusive bound.
    pub const fn right(self) -> u32 { self.right }

    /// Returns the rectangle width in pixels.
    pub const fn width(self) -> u32 { self.right - self.left }

    /// Returns the rectangle height in pixels.
    pub const fn height(self) -> u32 { self.bottom - self.top }

    /// Returns whether a stored coordinate lies inside the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{PixelCoord, PixelRect};
    ///
    /// let rect = PixelRect::new(1, 2, 4, 6).unwrap();
    /// assert!(rect.contains(PixelCoord::new(1, 2)));
    /// assert!(!rect.contains(PixelCoord::new(4, 2)));
    /// ```
    pub const fn contains(self, point: PixelCoord) -> bool {
        point.row >= self.top
            && point.row < self.bottom
            && point.col >= self.left
            && point.col < self.right
    }

    pub(crate) const fn is_inside(self, stored: PixelSize) -> bool {
        self.bottom <= stored.height && self.right <= stored.width
    }
}

/// The active image rectangle in stored sensor coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ActiveArea(PixelRect);

impl ActiveArea {
    /// Creates a non-empty ActiveArea contained in the stored plane.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidActiveArea`] when the rectangle is
    /// empty or extends beyond `stored`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{ActiveArea, PixelSize};
    ///
    /// let area = ActiveArea::try_new(PixelSize::new(8, 6).unwrap(), 1, 1, 5, 7).unwrap();
    /// assert_eq!(area.rect().width(), 6);
    /// ```
    pub fn try_new(
        stored: PixelSize,
        top: u32,
        left: u32,
        bottom: u32,
        right: u32,
    ) -> Result<Self, RawValidationError> {
        let rect = PixelRect::new(top, left, bottom, right)
            .filter(|rect| rect.is_inside(stored))
            .ok_or(RawValidationError::InvalidActiveArea)?;
        Ok(Self(rect))
    }

    /// Returns the ActiveArea as a half-open stored-pixel rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{ActiveArea, PixelSize};
    ///
    /// let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 0, 0, 4, 4).unwrap();
    /// assert_eq!(area.rect().bottom(), 4);
    /// ```
    pub const fn rect(self) -> PixelRect { self.0 }

    /// Returns whether a stored coordinate is inside the ActiveArea.
    pub const fn contains(self, point: PixelCoord) -> bool { self.0.contains(point) }
}

/// Stored geometry and original backend row stride.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StoredGeometry {
    dimensions: PixelSize,
    source_row_stride_bytes: NonZeroU64,
}

impl StoredGeometry {
    /// Creates stored geometry while preserving the backend's row stride.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU64;
    ///
    /// use fornax::geometry::{PixelSize, StoredGeometry};
    ///
    /// let geometry = StoredGeometry::new(PixelSize::new(4, 2).unwrap(), NonZeroU64::new(8).unwrap());
    /// assert_eq!(geometry.dimensions().height(), 2);
    /// ```
    pub const fn new(dimensions: PixelSize, source_row_stride_bytes: NonZeroU64) -> Self {
        Self {
            dimensions,
            source_row_stride_bytes,
        }
    }

    /// Returns compact stored dimensions.
    pub const fn dimensions(self) -> PixelSize { self.dimensions }

    /// Returns the original backend row stride in bytes.
    pub const fn source_row_stride_bytes(self) -> NonZeroU64 { self.source_row_stride_bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_area_is_half_open_and_bounded() {
        let stored = PixelSize::new(5, 4).unwrap();
        let area = ActiveArea::try_new(stored, 1, 2, 4, 5).unwrap();
        assert!(area.contains(PixelCoord::new(1, 2)));
        assert!(area.contains(PixelCoord::new(3, 4)));
        assert!(!area.contains(PixelCoord::new(4, 4)));
        assert!(!area.contains(PixelCoord::new(3, 5)));
    }

    #[test]
    fn active_area_rejects_empty_and_out_of_bounds_rectangles() {
        let stored = PixelSize::new(5, 4).unwrap();
        assert_eq!(
            ActiveArea::try_new(stored, 1, 1, 1, 3),
            Err(RawValidationError::InvalidActiveArea)
        );
        assert_eq!(
            ActiveArea::try_new(stored, 0, 0, 5, 6),
            Err(RawValidationError::InvalidActiveArea)
        );
    }
}
