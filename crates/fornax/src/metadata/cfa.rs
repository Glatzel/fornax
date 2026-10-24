//! CFA shapes, colors, Bayer phases, and ActiveArea-relative lookup.

use std::num::NonZeroU32;

use crate::error::RawValidationError;
use crate::geometry::{ActiveArea, PixelCoord};

/// The dimensions of a CFA repeat tile.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CfaShape {
    rows: NonZeroU32,
    columns: NonZeroU32,
}

impl CfaShape {
    /// The conventional 2x2 Bayer repeat.
    pub const BAYER_2X2: Self = Self {
        rows: NonZeroU32::MIN.saturating_add(1),
        columns: NonZeroU32::MIN.saturating_add(1),
    };

    /// Creates a non-empty CFA repeat shape.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidCfaShape`] when either dimension
    /// is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::CfaShape;
    ///
    /// assert_eq!(CfaShape::try_new(2, 2).unwrap().rows(), 2);
    /// assert!(CfaShape::try_new(0, 2).is_err());
    /// ```
    pub fn try_new(rows: u32, columns: u32) -> Result<Self, RawValidationError> {
        Ok(Self {
            rows: NonZeroU32::new(rows).ok_or(RawValidationError::InvalidCfaShape)?,
            columns: NonZeroU32::new(columns).ok_or(RawValidationError::InvalidCfaShape)?,
        })
    }

    /// Returns the repeat row count.
    pub const fn rows(self) -> u32 { self.rows.get() }

    /// Returns the repeat column count.
    pub const fn columns(self) -> u32 { self.columns.get() }
}

/// A CFA plane color or an unsupported observed source code.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CfaColor {
    /// Red.
    Red,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// A source color or plane code outside RGB.
    Other(u8),
}

/// A position in a 2x2 Bayer repeat tile.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BayerSlot(u8);

impl BayerSlot {
    /// Creates a Bayer slot from row and column phase bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::BayerSlot;
    ///
    /// assert_eq!(BayerSlot::from_local(3, 4).index(), 2);
    /// ```
    pub const fn from_local(row: u32, column: u32) -> Self {
        Self((((row & 1) << 1) | (column & 1)) as u8)
    }

    /// Returns the row-major slot index in `0..=3`.
    pub const fn index(self) -> u8 { self.0 }

    /// Returns the tile-local row.
    pub const fn row(self) -> u32 { ((self.0 >> 1) & 1) as u32 }

    /// Returns the tile-local column.
    pub const fn column(self) -> u32 { (self.0 & 1) as u32 }
}

/// A conventional 2x2 RGB Bayer phase order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum BayerPattern {
    /// Red, green, green, blue.
    Rggb,
    /// Blue, green, green, red.
    Bggr,
    /// Green, red, blue, green.
    Grbg,
    /// Green, blue, red, green.
    Gbrg,
}

impl BayerPattern {
    /// Converts an observed CFA tile to a supported Bayer phase.
    ///
    /// Monochrome, non-2x2, and non-RGB layouts produce typed errors.
    ///
    /// # Errors
    ///
    /// Returns a structured unsupported-layout error for anything other than a
    /// conventional 2x2 RGB Bayer tile.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::{BayerPattern, CfaColor, CfaShape};
    ///
    /// let colors = [
    ///     CfaColor::Red,
    ///     CfaColor::Green,
    ///     CfaColor::Green,
    ///     CfaColor::Blue,
    /// ];
    /// assert_eq!(
    ///     BayerPattern::try_from_cfa(CfaShape::BAYER_2X2, colors).unwrap(),
    ///     BayerPattern::Rggb
    /// );
    /// ```
    pub fn try_from_cfa(
        shape: CfaShape,
        colors: [CfaColor; 4],
    ) -> Result<Self, RawValidationError> {
        if shape.rows() == 1 && shape.columns() == 1 {
            return Err(RawValidationError::UnsupportedNonMosaic);
        }
        if shape != CfaShape::BAYER_2X2 {
            return Err(RawValidationError::UnsupportedCfa {
                rows: shape.rows(),
                columns: shape.columns(),
            });
        }
        if let Some(code) = colors.iter().find_map(|color| match color {
            CfaColor::Other(code) => Some(*code),
            CfaColor::Red | CfaColor::Green | CfaColor::Blue => None,
        }) {
            return Err(RawValidationError::UnsupportedNonRgb { color_code: code });
        }

        match colors {
            [
                CfaColor::Red,
                CfaColor::Green,
                CfaColor::Green,
                CfaColor::Blue,
            ] => Ok(Self::Rggb),
            [
                CfaColor::Blue,
                CfaColor::Green,
                CfaColor::Green,
                CfaColor::Red,
            ] => Ok(Self::Bggr),
            [
                CfaColor::Green,
                CfaColor::Red,
                CfaColor::Blue,
                CfaColor::Green,
            ] => Ok(Self::Grbg),
            [
                CfaColor::Green,
                CfaColor::Blue,
                CfaColor::Red,
                CfaColor::Green,
            ] => Ok(Self::Gbrg),
            _ => Err(RawValidationError::InvalidCfa),
        }
    }

    /// Returns the conventional Bayer repeat shape.
    pub const fn shape(self) -> CfaShape { CfaShape::BAYER_2X2 }

    /// Returns the color represented by a Bayer slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::{BayerPattern, BayerSlot, CfaColor};
    ///
    /// assert_eq!(
    ///     BayerPattern::Grbg.color_at(BayerSlot::from_local(1, 0)),
    ///     CfaColor::Blue
    /// );
    /// ```
    pub const fn color_at(self, slot: BayerSlot) -> CfaColor {
        match (self, slot.index()) {
            (Self::Rggb, 0) | (Self::Bggr, 3) | (Self::Grbg, 1) | (Self::Gbrg, 2) => CfaColor::Red,
            (Self::Rggb, 3) | (Self::Bggr, 0) | (Self::Grbg, 2) | (Self::Gbrg, 1) => CfaColor::Blue,
            _ => CfaColor::Green,
        }
    }
}

/// A Bayer phase anchored at an ActiveArea's top-left pixel.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BayerLayout {
    pattern: BayerPattern,
    active_area: ActiveArea,
}

impl BayerLayout {
    /// Creates a Bayer layout anchored at `active_area`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{ActiveArea, PixelSize};
    /// use fornax::metadata::{BayerLayout, BayerPattern};
    ///
    /// let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 0, 0, 4, 4).unwrap();
    /// let layout = BayerLayout::new(BayerPattern::Rggb, area);
    /// assert_eq!(layout.pattern(), BayerPattern::Rggb);
    /// ```
    pub const fn new(pattern: BayerPattern, active_area: ActiveArea) -> Self {
        Self {
            pattern,
            active_area,
        }
    }

    /// Returns the color phase.
    pub const fn pattern(self) -> BayerPattern { self.pattern }

    /// Returns the coordinate anchor for the CFA phase.
    pub const fn active_area(self) -> ActiveArea { self.active_area }

    /// Returns the Bayer slot at a stored sensor coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::CoordOutsideActiveArea`] outside the
    /// ActiveArea.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{ActiveArea, PixelCoord, PixelSize};
    /// use fornax::metadata::{BayerLayout, BayerPattern};
    ///
    /// let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 1, 1, 4, 4).unwrap();
    /// let layout = BayerLayout::new(BayerPattern::Gbrg, area);
    /// assert_eq!(layout.slot_at(PixelCoord::new(1, 1)).unwrap().index(), 0);
    /// ```
    pub fn slot_at(&self, stored: PixelCoord) -> Result<BayerSlot, RawValidationError> {
        if !self.active_area.contains(stored) {
            return Err(RawValidationError::CoordOutsideActiveArea);
        }
        let rect = self.active_area.rect();
        Ok(BayerSlot::from_local(
            stored.row - rect.top(),
            stored.col - rect.left(),
        ))
    }

    /// Returns the CFA color at a stored sensor coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::CoordOutsideActiveArea`] outside the
    /// ActiveArea.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::geometry::{ActiveArea, PixelCoord, PixelSize};
    /// use fornax::metadata::{BayerLayout, BayerPattern, CfaColor};
    ///
    /// let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 0, 0, 4, 4).unwrap();
    /// let layout = BayerLayout::new(BayerPattern::Bggr, area);
    /// assert_eq!(layout.color_at(PixelCoord::new(1, 1)), Ok(CfaColor::Red));
    /// ```
    pub fn color_at(&self, stored: PixelCoord) -> Result<CfaColor, RawValidationError> {
        self.slot_at(stored).map(|slot| self.pattern.color_at(slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::PixelSize;

    const PATTERNS: [BayerPattern; 4] = [
        BayerPattern::Rggb,
        BayerPattern::Bggr,
        BayerPattern::Grbg,
        BayerPattern::Gbrg,
    ];

    const COLORS: [[CfaColor; 4]; 4] = [
        [
            CfaColor::Red,
            CfaColor::Green,
            CfaColor::Green,
            CfaColor::Blue,
        ],
        [
            CfaColor::Blue,
            CfaColor::Green,
            CfaColor::Green,
            CfaColor::Red,
        ],
        [
            CfaColor::Green,
            CfaColor::Red,
            CfaColor::Blue,
            CfaColor::Green,
        ],
        [
            CfaColor::Green,
            CfaColor::Blue,
            CfaColor::Red,
            CfaColor::Green,
        ],
    ];

    #[test]
    fn every_pattern_maps_its_four_slots() {
        for (pattern, expected) in PATTERNS.into_iter().zip(COLORS) {
            for row in 0..2 {
                for column in 0..2 {
                    let slot = BayerSlot::from_local(row, column);
                    assert_eq!(
                        pattern.color_at(slot),
                        expected[slot.index() as usize],
                        "{pattern:?} at ({row}, {column})"
                    );
                }
            }
        }
    }

    #[test]
    fn active_relative_phase_is_preserved_at_odd_origins() {
        for pattern in PATTERNS {
            for (top, left) in [(0, 0), (1, 0), (0, 1), (1, 1), (3, 5)] {
                let stored = PixelSize::new(12, 10).unwrap();
                let area = ActiveArea::try_new(stored, top, left, 10, 11).unwrap();
                let layout = BayerLayout::new(pattern, area);
                let first = layout.slot_at(PixelCoord::new(top, left)).unwrap();
                assert_eq!(first.index(), 0);
                for row in top..10 {
                    for column in left..11 {
                        let slot = layout.slot_at(PixelCoord::new(row, column)).unwrap();
                        assert_eq!(
                            slot.index(),
                            ((((row - top) & 1) << 1) | ((column - left) & 1)) as u8
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn lookup_outside_active_area_is_structured() {
        let stored = PixelSize::new(8, 8).unwrap();
        let area = ActiveArea::try_new(stored, 2, 3, 7, 8).unwrap();
        let layout = BayerLayout::new(BayerPattern::Rggb, area);
        for point in [
            PixelCoord::new(1, 3),
            PixelCoord::new(2, 2),
            PixelCoord::new(7, 7),
            PixelCoord::new(6, 8),
        ] {
            assert_eq!(
                layout.slot_at(point),
                Err(RawValidationError::CoordOutsideActiveArea)
            );
        }
    }

    #[test]
    fn unsupported_cfa_shapes_and_colors_are_typed() {
        assert_eq!(
            BayerPattern::try_from_cfa(CfaShape::try_new(1, 1).unwrap(), [CfaColor::Red; 4]),
            Err(RawValidationError::UnsupportedNonMosaic)
        );
        assert_eq!(
            BayerPattern::try_from_cfa(CfaShape::try_new(4, 4).unwrap(), [CfaColor::Red; 4]),
            Err(RawValidationError::UnsupportedCfa {
                rows: 4,
                columns: 4
            })
        );
        assert_eq!(
            BayerPattern::try_from_cfa(
                CfaShape::BAYER_2X2,
                [
                    CfaColor::Red,
                    CfaColor::Green,
                    CfaColor::Other(3),
                    CfaColor::Blue
                ]
            ),
            Err(RawValidationError::UnsupportedNonRgb { color_code: 3 })
        );
    }
}
