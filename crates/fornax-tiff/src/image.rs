//! Typed multi-channel and multi-plane TIFF image types.

use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use fornax_codec::{CodecError, SampleBufferError};
use fornax_core::{MonoImage, MultiPlaneImage, RgbImage, RgbaImage, UvImage};

use crate::{
    RasterError, TiffCompressionError, TiffError, TiffSource, TiffWriteOptions, TiffWriter,
};

/// A typed image buffer matching one of `fornax-core`'s image layouts.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedTiffImage<T: image::Primitive> {
    /// Single-channel image.
    Mono(MonoImage<T>),
    /// Two-channel image (for example UV or grayscale + alpha).
    Uv(UvImage<T>),
    /// Three-channel RGB image.
    Rgb(RgbImage<T>),
    /// Four-channel RGBA image.
    Rgba(RgbaImage<T>),
    /// Arbitrary-channel planar image (`Vec<MonoImage<T>>`).
    MultiPlane(MultiPlaneImage<T>),
}

impl<T: image::Primitive> TypedTiffImage<T> {
    /// Returns the image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Mono(image) => image.width(),
            Self::Uv(image) => image.width(),
            Self::Rgb(image) => image.width(),
            Self::Rgba(image) => image.width(),
            Self::MultiPlane(planes) => planes.first().map_or(0, MonoImage::width),
        }
    }

    /// Returns the image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Mono(image) => image.height(),
            Self::Uv(image) => image.height(),
            Self::Rgb(image) => image.height(),
            Self::Rgba(image) => image.height(),
            Self::MultiPlane(planes) => planes.first().map_or(0, MonoImage::height),
        }
    }

    /// Returns the number of channels (samples per pixel).
    pub fn samples_per_pixel(&self) -> usize {
        match self {
            Self::Mono(_) => 1,
            Self::Uv(_) => 2,
            Self::Rgb(_) => 3,
            Self::Rgba(_) => 4,
            Self::MultiPlane(planes) => planes.len(),
        }
    }
}

impl<T: image::Primitive> From<MonoImage<T>> for TypedTiffImage<T> {
    fn from(image: MonoImage<T>) -> Self { Self::Mono(image) }
}

impl<T: image::Primitive> From<UvImage<T>> for TypedTiffImage<T> {
    fn from(image: UvImage<T>) -> Self { Self::Uv(image) }
}

impl<T: image::Primitive> From<RgbImage<T>> for TypedTiffImage<T> {
    fn from(image: RgbImage<T>) -> Self { Self::Rgb(image) }
}

impl<T: image::Primitive> From<RgbaImage<T>> for TypedTiffImage<T> {
    fn from(image: RgbaImage<T>) -> Self { Self::Rgba(image) }
}

impl<T: image::Primitive> From<MultiPlaneImage<T>> for TypedTiffImage<T> {
    fn from(planes: MultiPlaneImage<T>) -> Self { Self::MultiPlane(planes) }
}

/// Sample-type-erased TIFF image supporting mono, UV, RGB, RGBA, and
/// multi-plane layouts.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TiffImage {
    /// 8-bit unsigned samples.
    U8(TypedTiffImage<u8>),
    /// 16-bit unsigned samples.
    U16(TypedTiffImage<u16>),
    /// 32-bit unsigned samples.
    U32(TypedTiffImage<u32>),
    /// 32-bit floating-point samples.
    F32(TypedTiffImage<f32>),
    /// 64-bit floating-point samples.
    F64(TypedTiffImage<f64>),
}

impl TiffImage {
    /// Decodes the root directory of an in-memory TIFF container.
    ///
    /// Channel count mapping:
    /// - `1` -> [`TypedTiffImage::Mono`]
    /// - `2` -> [`TypedTiffImage::Uv`]
    /// - `3` -> [`TypedTiffImage::Rgb`]
    /// - `4` -> [`TypedTiffImage::Rgba`]
    /// - `>= 5` -> [`TypedTiffImage::MultiPlane`]
    pub fn from_bytes(source: &[u8]) -> Result<Self, TiffIoError> {
        crate::parse(source)?.decode_image(0)
    }

    /// Decodes the root directory of a TIFF file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TiffIoError> {
        let source = TiffSource::from_path(path.as_ref())?;
        crate::parse_source(source)?.decode_image(0)
    }

    /// Writes this image to a seekable writer using `options`.
    pub fn write_to<W: Write + Seek>(
        &self,
        writer: W,
        options: &TiffWriteOptions,
    ) -> Result<(), TiffIoError> {
        TiffWriter::new(writer)?.write_image(self, options)
    }

    /// Writes this image to a file at `path` using `options`.
    pub fn save(
        &self,
        path: impl AsRef<Path>,
        options: &TiffWriteOptions,
    ) -> Result<(), TiffIoError> {
        let file = File::create(path.as_ref())?;
        self.write_to(BufWriter::new(file), options)
    }

    /// Returns the image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::U8(image) => image.width(),
            Self::U16(image) => image.width(),
            Self::U32(image) => image.width(),
            Self::F32(image) => image.width(),
            Self::F64(image) => image.width(),
        }
    }

    /// Returns the image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::U8(image) => image.height(),
            Self::U16(image) => image.height(),
            Self::U32(image) => image.height(),
            Self::F32(image) => image.height(),
            Self::F64(image) => image.height(),
        }
    }

    /// Returns the number of channels (samples per pixel).
    pub fn samples_per_pixel(&self) -> usize {
        match self {
            Self::U8(image) => image.samples_per_pixel(),
            Self::U16(image) => image.samples_per_pixel(),
            Self::U32(image) => image.samples_per_pixel(),
            Self::F32(image) => image.samples_per_pixel(),
            Self::F64(image) => image.samples_per_pixel(),
        }
    }
}

macro_rules! impl_from_image_for_tiff_image {
    ($sample:ty, $variant:ident) => {
        impl From<TypedTiffImage<$sample>> for TiffImage {
            fn from(image: TypedTiffImage<$sample>) -> Self { Self::$variant(image) }
        }

        impl From<MonoImage<$sample>> for TiffImage {
            fn from(image: MonoImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::Mono(image))
            }
        }

        impl From<UvImage<$sample>> for TiffImage {
            fn from(image: UvImage<$sample>) -> Self { Self::$variant(TypedTiffImage::Uv(image)) }
        }

        impl From<RgbImage<$sample>> for TiffImage {
            fn from(image: RgbImage<$sample>) -> Self { Self::$variant(TypedTiffImage::Rgb(image)) }
        }

        impl From<RgbaImage<$sample>> for TiffImage {
            fn from(image: RgbaImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::Rgba(image))
            }
        }

        impl From<MultiPlaneImage<$sample>> for TiffImage {
            fn from(planes: MultiPlaneImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::MultiPlane(planes))
            }
        }
    };
}

impl_from_image_for_tiff_image!(u8, U8);
impl_from_image_for_tiff_image!(u16, U16);
impl_from_image_for_tiff_image!(u32, U32);
impl_from_image_for_tiff_image!(f32, F32);
impl_from_image_for_tiff_image!(f64, F64);

/// Failure during high-level TIFF image reading or writing.
#[derive(Debug)]
pub enum TiffIoError {
    /// Low-level TIFF container parsing or writing failure.
    Tiff(TiffError),
    /// Raster layout validation failure.
    Raster(RasterError),
    /// Compression identifier resolution failure.
    Compression(TiffCompressionError),
    /// Codec decode or encode failure.
    Codec(CodecError),
    /// File system I/O failure.
    Io(std::io::Error),
    /// Malformed or inconsistent TIFF image metadata.
    Invalid(Box<str>),
    /// Recognized TIFF feature outside the supported image IO scope.
    Unsupported(Box<str>),
}

impl fmt::Display for TiffIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tiff(error) => write!(formatter, "{error}"),
            Self::Raster(error) => write!(formatter, "{error}"),
            Self::Compression(error) => write!(formatter, "{error}"),
            Self::Codec(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Invalid(detail) | Self::Unsupported(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for TiffIoError {}

impl From<TiffError> for TiffIoError {
    fn from(error: TiffError) -> Self { Self::Tiff(error) }
}

impl From<RasterError> for TiffIoError {
    fn from(error: RasterError) -> Self { Self::Raster(error) }
}

impl From<TiffCompressionError> for TiffIoError {
    fn from(error: TiffCompressionError) -> Self { Self::Compression(error) }
}

impl From<CodecError> for TiffIoError {
    fn from(error: CodecError) -> Self { Self::Codec(error) }
}

impl From<SampleBufferError> for TiffIoError {
    fn from(error: SampleBufferError) -> Self { Self::Codec(error.into()) }
}

impl From<std::io::Error> for TiffIoError {
    fn from(error: std::io::Error) -> Self { Self::Io(error) }
}

pub(crate) fn deinterleave_planes<T: image::Primitive>(
    width: u32,
    height: u32,
    samples_per_pixel: usize,
    interleaved: &[T],
) -> Result<MultiPlaneImage<T>, TiffIoError> {
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| TiffIoError::Invalid("pixel count overflow".into()))?;
    let mut planes = (0..samples_per_pixel)
        .map(|_| Vec::with_capacity(pixel_count))
        .collect::<Vec<_>>();
    for pixel in interleaved.chunks_exact(samples_per_pixel) {
        for (plane, &sample) in planes.iter_mut().zip(pixel) {
            plane.push(sample);
        }
    }
    planes
        .into_iter()
        .map(|raw| {
            MonoImage::from_raw(width, height, raw)
                .ok_or_else(|| TiffIoError::Invalid("invalid deinterleaved plane buffer".into()))
        })
        .collect()
}

pub(crate) fn interleave_planes<T: image::Primitive>(planes: &[MonoImage<T>]) -> Vec<T> {
    let pixel_count = planes.first().map_or(0, |p| p.as_raw().len());
    let mut interleaved = Vec::with_capacity(pixel_count * planes.len());
    for idx in 0..pixel_count {
        for plane in planes {
            interleaved.push(plane.as_raw()[idx]);
        }
    }
    interleaved
}
