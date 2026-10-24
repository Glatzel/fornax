use image::{ImageBuffer, Luma};

use crate::{PixelSize, RawValidationError};

/// A tightly packed single-channel 8-bit RAW plane.
pub type U8Plane = ImageBuffer<Luma<u8>, Vec<u8>>;
/// A tightly packed single-channel 16-bit RAW plane.
pub type U16Plane = ImageBuffer<Luma<u16>, Vec<u16>>;
/// A tightly packed single-channel 32-bit RAW plane.
pub type U32Plane = ImageBuffer<Luma<u32>, Vec<u32>>;
/// A tightly packed single-channel `f32` RAW plane.
pub type F32Plane = ImageBuffer<Luma<f32>, Vec<f32>>;

/// The semantic sample format declared by a source file or decoder.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SampleFormat {
    /// Unsigned integer samples.
    UnsignedInteger,
    /// Floating-point samples represented by the initial `f32` path.
    Float,
}

/// Declared precision and format, independent of the Rust storage container.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SampleEncoding {
    bits_per_sample: u8,
    format: SampleFormat,
}

impl SampleEncoding {
    /// Creates a supported sample encoding.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidSampleEncoding`] for zero bits and
    /// [`RawValidationError::UnsupportedSampleEncoding`] above 32 bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{SampleEncoding, SampleFormat};
    ///
    /// let encoding = SampleEncoding::try_new(12, SampleFormat::UnsignedInteger).unwrap();
    /// assert_eq!(encoding.bits_per_sample(), 12);
    /// ```
    pub fn try_new(bits_per_sample: u8, format: SampleFormat) -> Result<Self, RawValidationError> {
        if bits_per_sample == 0 {
            return Err(RawValidationError::InvalidSampleEncoding);
        }
        if bits_per_sample > 32 {
            return Err(RawValidationError::UnsupportedSampleEncoding {
                bits_per_sample: u16::from(bits_per_sample),
                format,
            });
        }
        Ok(Self {
            bits_per_sample,
            format,
        })
    }

    /// Returns the declared semantic precision in bits.
    pub const fn bits_per_sample(self) -> u8 { self.bits_per_sample }

    /// Returns the declared sample format.
    pub const fn format(self) -> SampleFormat { self.format }

    pub(crate) const fn is_f32_exact(self) -> bool {
        match self.format {
            SampleFormat::UnsignedInteger => self.bits_per_sample <= 24,
            SampleFormat::Float => true,
        }
    }
}

/// The native storage representation of decoded sensor samples.
#[derive(Clone, Debug, PartialEq)]
pub enum StoredPlane {
    /// 8-bit unsigned samples.
    U8(U8Plane),
    /// 16-bit unsigned samples.
    U16(U16Plane),
    /// 32-bit unsigned samples.
    U32(U32Plane),
    /// Floating-point samples.
    F32(F32Plane),
}

impl StoredPlane {
    /// Returns the plane's stored dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidGeometry`] for a zero dimension.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{PixelSize, U8Plane};
    ///
    /// let plane = U8Plane::from_raw(2, 1, vec![1, 2]).unwrap();
    /// assert_eq!(
    ///     fornax_core::StoredPlane::U8(plane).dimensions().unwrap(),
    ///     PixelSize::new(2, 1).unwrap()
    /// );
    /// ```
    pub fn dimensions(&self) -> Result<PixelSize, RawValidationError> {
        let (width, height) = match self {
            Self::U8(plane) => plane.dimensions(),
            Self::U16(plane) => plane.dimensions(),
            Self::U32(plane) => plane.dimensions(),
            Self::F32(plane) => plane.dimensions(),
        };
        PixelSize::new(width, height).ok_or(RawValidationError::InvalidGeometry)
    }

    /// Validates native container width, format, exact length, and finiteness.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the container is incompatible with
    /// `encoding` or contains an invalid value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{SampleEncoding, SampleFormat, StoredPlane, U16Plane};
    ///
    /// let plane = StoredPlane::U16(U16Plane::from_raw(1, 1, vec![1023]).unwrap());
    /// let encoding = SampleEncoding::try_new(10, SampleFormat::UnsignedInteger).unwrap();
    /// assert!(plane.validate_native_encoding(encoding).is_ok());
    /// ```
    pub fn validate_native_encoding(
        &self,
        encoding: SampleEncoding,
    ) -> Result<(), RawValidationError> {
        let dimensions = self.dimensions()?;
        dimensions.sample_count()?;
        match (self, encoding.format, encoding.bits_per_sample) {
            (Self::U8(plane), SampleFormat::UnsignedInteger, 1..=8) => {
                validate_exact_len(plane.as_raw().len(), dimensions)
            }
            (Self::U16(plane), SampleFormat::UnsignedInteger, 9..=16) => {
                validate_exact_len(plane.as_raw().len(), dimensions)
            }
            (Self::U32(plane), SampleFormat::UnsignedInteger, 17..=32) => {
                validate_exact_len(plane.as_raw().len(), dimensions)
            }
            (Self::F32(plane), SampleFormat::Float, 1..=32) => {
                validate_exact_len(plane.as_raw().len(), dimensions)?;
                validate_finite(plane)
            }
            (_, format, bits_per_sample) => Err(RawValidationError::UnsupportedSampleEncoding {
                bits_per_sample: u16::from(bits_per_sample),
                format,
            }),
        }
    }

    pub(crate) fn into_f32(
        self,
        encoding: SampleEncoding,
        allow_f32_working_encoding: bool,
    ) -> Result<F32Plane, RawValidationError> {
        if !encoding.is_f32_exact() {
            return Err(RawValidationError::UnsupportedSampleEncoding {
                bits_per_sample: u16::from(encoding.bits_per_sample),
                format: encoding.format,
            });
        }

        match self {
            Self::U8(plane) => {
                if encoding.format != SampleFormat::UnsignedInteger || encoding.bits_per_sample > 8
                {
                    return Err(RawValidationError::UnsupportedPlaneEncoding);
                }
                validate_exact_len(plane.as_raw().len(), dimensions_from(plane.dimensions())?)?;
                let (width, height) = plane.dimensions();
                let samples = plane.into_raw().into_iter().map(f32::from).collect();
                ImageBuffer::from_raw(width, height, samples)
                    .ok_or(RawValidationError::InvalidGeometry)
            }
            Self::U16(plane) => {
                if encoding.format != SampleFormat::UnsignedInteger
                    || !(9..=16).contains(&encoding.bits_per_sample)
                {
                    return Err(RawValidationError::UnsupportedPlaneEncoding);
                }
                validate_exact_len(plane.as_raw().len(), dimensions_from(plane.dimensions())?)?;
                let (width, height) = plane.dimensions();
                let samples = plane.into_raw().into_iter().map(f32::from).collect();
                ImageBuffer::from_raw(width, height, samples)
                    .ok_or(RawValidationError::InvalidGeometry)
            }
            Self::U32(plane) => {
                if encoding.format != SampleFormat::UnsignedInteger
                    || !(17..=32).contains(&encoding.bits_per_sample)
                {
                    return Err(RawValidationError::UnsupportedPlaneEncoding);
                }
                validate_exact_len(plane.as_raw().len(), dimensions_from(plane.dimensions())?)?;
                let (width, height) = plane.dimensions();
                let samples = plane
                    .into_raw()
                    .into_iter()
                    .map(|sample| sample as f32)
                    .collect();
                ImageBuffer::from_raw(width, height, samples)
                    .ok_or(RawValidationError::InvalidGeometry)
            }
            Self::F32(plane) => {
                let valid = if allow_f32_working_encoding {
                    encoding.is_f32_exact()
                } else {
                    encoding.format == SampleFormat::Float
                };
                if !valid {
                    return Err(RawValidationError::UnsupportedPlaneEncoding);
                }
                validate_exact_len(plane.as_raw().len(), dimensions_from(plane.dimensions())?)?;
                validate_finite(&plane)?;
                Ok(plane)
            }
        }
    }
}

fn dimensions_from((width, height): (u32, u32)) -> Result<PixelSize, RawValidationError> {
    PixelSize::new(width, height).ok_or(RawValidationError::InvalidGeometry)
}

fn validate_exact_len(actual: usize, dimensions: PixelSize) -> Result<(), RawValidationError> {
    if actual == dimensions.sample_count()? {
        Ok(())
    } else {
        Err(RawValidationError::InvalidGeometry)
    }
}

pub(crate) fn validate_finite(plane: &F32Plane) -> Result<(), RawValidationError> {
    if plane.as_raw().iter().all(|sample| sample.is_finite()) {
        Ok(())
    } else {
        Err(RawValidationError::NonFiniteSample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoding(bits: u8, format: SampleFormat) -> SampleEncoding {
        SampleEncoding::try_new(bits, format).unwrap()
    }

    #[test]
    fn native_containers_reject_semantically_invalid_widths() {
        let u8_plane = StoredPlane::U8(U8Plane::from_raw(1, 1, vec![7]).unwrap());
        let u16_plane = StoredPlane::U16(U16Plane::from_raw(1, 1, vec![7]).unwrap());
        let u32_plane = StoredPlane::U32(U32Plane::from_raw(1, 1, vec![7]).unwrap());
        let f32_plane = StoredPlane::F32(F32Plane::from_raw(1, 1, vec![0.5]).unwrap());

        assert!(
            u8_plane
                .validate_native_encoding(encoding(9, SampleFormat::UnsignedInteger))
                .is_err()
        );
        assert!(
            u16_plane
                .validate_native_encoding(encoding(8, SampleFormat::UnsignedInteger))
                .is_err()
        );
        assert!(
            u32_plane
                .validate_native_encoding(encoding(16, SampleFormat::UnsignedInteger))
                .is_err()
        );
        assert!(
            f32_plane
                .validate_native_encoding(encoding(32, SampleFormat::UnsignedInteger))
                .is_err()
        );
        assert_eq!(
            SampleEncoding::try_new(33, SampleFormat::UnsignedInteger),
            Err(RawValidationError::UnsupportedSampleEncoding {
                bits_per_sample: 33,
                format: SampleFormat::UnsignedInteger,
            })
        );
    }

    #[test]
    fn oversized_buffers_are_rejected_instead_of_reinterpreted() {
        let plane = StoredPlane::U8(U8Plane::from_raw(1, 1, vec![1, 2]).unwrap());
        assert_eq!(
            plane.validate_native_encoding(encoding(8, SampleFormat::UnsignedInteger)),
            Err(RawValidationError::InvalidGeometry)
        );
    }

    #[test]
    fn u8_u16_and_supported_u32_convert_exactly_to_f32() {
        for sample in 0..=u8::MAX {
            let plane = StoredPlane::U8(U8Plane::from_raw(1, 1, vec![sample]).unwrap());
            let converted = plane
                .into_f32(encoding(8, SampleFormat::UnsignedInteger), false)
                .unwrap();
            assert_eq!(converted.get_pixel(0, 0)[0], f32::from(sample));
        }
        for sample in 0..=u16::MAX {
            let plane = StoredPlane::U16(U16Plane::from_raw(1, 1, vec![sample]).unwrap());
            let converted = plane
                .into_f32(encoding(16, SampleFormat::UnsignedInteger), false)
                .unwrap();
            assert_eq!(converted.get_pixel(0, 0)[0], f32::from(sample));
        }

        let sample = (1_u32 << 24) - 1;
        let plane = StoredPlane::U32(U32Plane::from_raw(1, 1, vec![sample]).unwrap());
        assert_eq!(
            plane
                .into_f32(encoding(24, SampleFormat::UnsignedInteger), false)
                .unwrap()
                .get_pixel(0, 0)[0],
            sample as f32
        );
    }

    #[test]
    fn wider_u32_and_non_finite_floats_are_rejected() {
        let plane = StoredPlane::U32(U32Plane::from_raw(1, 1, vec![u32::MAX]).unwrap());
        assert!(
            plane
                .into_f32(encoding(32, SampleFormat::UnsignedInteger), false)
                .is_err()
        );
        let plane = StoredPlane::F32(F32Plane::from_raw(1, 1, vec![f32::NAN]).unwrap());
        assert_eq!(
            plane.validate_native_encoding(encoding(32, SampleFormat::Float)),
            Err(RawValidationError::NonFiniteSample)
        );
    }
}
