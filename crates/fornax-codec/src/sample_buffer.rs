use image::{ImageBuffer, Luma};
use thiserror::Error;

type Mono<T> = ImageBuffer<Luma<T>, Vec<T>>;

/// A single-band buffer of raw codec samples.
///
/// `Luma` stores one scalar channel. Its samples are not necessarily
/// luminance values.
///
/// # Examples
///
/// ```
/// use fornax_codec::SampleBuffer;
///
/// let buffer = SampleBuffer::from_u16(2, 1, vec![10, 20]).unwrap();
/// assert_eq!((buffer.width(), buffer.height()), (2, 1));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SampleBuffer {
    /// 8-bit unsigned samples.
    U8(Mono<u8>),
    /// 16-bit unsigned samples.
    U16(Mono<u16>),
    /// 32-bit unsigned samples.
    U32(Mono<u32>),
    /// 32-bit floating-point samples.
    F32(Mono<f32>),
}

impl SampleBuffer {
    /// Creates an 8-bit unsigned sample buffer.
    ///
    /// # Errors
    ///
    /// Returns [`SampleBufferError`] when the dimensions overflow the
    /// platform's address space, the dimensions are invalid, or `data` does
    /// not contain exactly `width * height` samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u8(2, 1, vec![1, 2]).unwrap();
    /// assert_eq!(buffer.into_raw_u8(), Some(vec![1, 2]));
    /// ```
    pub fn from_u8(width: u32, height: u32, data: Vec<u8>) -> Result<Self, SampleBufferError> {
        create(width, height, data).map(Self::U8)
    }

    /// Creates a 16-bit unsigned sample buffer.
    ///
    /// # Errors
    ///
    /// Returns [`SampleBufferError`] when the dimensions overflow the
    /// platform's address space, the dimensions are invalid, or `data` does
    /// not contain exactly `width * height` samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u16(2, 1, vec![1, 65_535]).unwrap();
    /// assert_eq!(buffer.into_raw_u16(), Some(vec![1, 65_535]));
    /// ```
    pub fn from_u16(width: u32, height: u32, data: Vec<u16>) -> Result<Self, SampleBufferError> {
        create(width, height, data).map(Self::U16)
    }

    /// Creates a 32-bit unsigned sample buffer.
    ///
    /// # Errors
    ///
    /// Returns [`SampleBufferError`] when the dimensions overflow the
    /// platform's address space, the dimensions are invalid, or `data` does
    /// not contain exactly `width * height` samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u32(1, 1, vec![4_000_000_000]).unwrap();
    /// assert_eq!(buffer.into_raw_u32(), Some(vec![4_000_000_000]));
    /// ```
    pub fn from_u32(width: u32, height: u32, data: Vec<u32>) -> Result<Self, SampleBufferError> {
        create(width, height, data).map(Self::U32)
    }

    /// Creates a 32-bit floating-point sample buffer.
    ///
    /// Sample values and their bit patterns are preserved.
    ///
    /// # Errors
    ///
    /// Returns [`SampleBufferError`] when the dimensions overflow the
    /// platform's address space, the dimensions are invalid, or `data` does
    /// not contain exactly `width * height` samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_f32(2, 1, vec![-1.5, 0.25]).unwrap();
    /// assert_eq!(buffer.into_raw_f32(), Some(vec![-1.5, 0.25]));
    /// ```
    pub fn from_f32(width: u32, height: u32, data: Vec<f32>) -> Result<Self, SampleBufferError> {
        create(width, height, data).map(Self::F32)
    }

    /// Returns the image width in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u8(3, 2, vec![0; 6]).unwrap();
    /// assert_eq!(buffer.width(), 3);
    /// ```
    pub fn width(&self) -> u32 {
        match self {
            Self::U8(image) => image.width(),
            Self::U16(image) => image.width(),
            Self::U32(image) => image.width(),
            Self::F32(image) => image.width(),
        }
    }

    /// Returns the image height in samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u8(3, 2, vec![0; 6]).unwrap();
    /// assert_eq!(buffer.height(), 2);
    /// ```
    pub fn height(&self) -> u32 {
        match self {
            Self::U8(image) => image.height(),
            Self::U16(image) => image.height(),
            Self::U32(image) => image.height(),
            Self::F32(image) => image.height(),
        }
    }

    /// Consumes the buffer and returns its underlying 8-bit samples.
    ///
    /// Returns `None` when the buffer contains another sample type. A
    /// successful extraction does not copy the sample vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u8(2, 1, vec![1, 2]).unwrap();
    /// assert_eq!(buffer.into_raw_u8(), Some(vec![1, 2]));
    /// ```
    pub fn into_raw_u8(self) -> Option<Vec<u8>> {
        match self {
            Self::U8(image) => Some(image.into_raw()),
            Self::U16(_) | Self::U32(_) | Self::F32(_) => None,
            _ => None,
        }
    }

    /// Consumes the buffer and returns its underlying 16-bit samples.
    ///
    /// Returns `None` when the buffer contains another sample type. A
    /// successful extraction does not copy the sample vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u16(2, 1, vec![1, 2]).unwrap();
    /// assert_eq!(buffer.into_raw_u16(), Some(vec![1, 2]));
    /// ```
    pub fn into_raw_u16(self) -> Option<Vec<u16>> {
        match self {
            Self::U16(image) => Some(image.into_raw()),
            Self::U8(_) | Self::U32(_) | Self::F32(_) => None,
            _ => None,
        }
    }

    /// Consumes the buffer and returns its underlying 32-bit unsigned samples.
    ///
    /// Returns `None` when the buffer contains another sample type. A
    /// successful extraction does not copy the sample vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_u32(2, 1, vec![1, 2]).unwrap();
    /// assert_eq!(buffer.into_raw_u32(), Some(vec![1, 2]));
    /// ```
    pub fn into_raw_u32(self) -> Option<Vec<u32>> {
        match self {
            Self::U32(image) => Some(image.into_raw()),
            Self::U8(_) | Self::U16(_) | Self::F32(_) => None,
            _ => None,
        }
    }

    /// Consumes the buffer and returns its underlying 32-bit float samples.
    ///
    /// Returns `None` when the buffer contains another sample type. A
    /// successful extraction does not copy the sample vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::from_f32(2, 1, vec![1.0, 2.0]).unwrap();
    /// assert_eq!(buffer.into_raw_f32(), Some(vec![1.0, 2.0]));
    /// ```
    pub fn into_raw_f32(self) -> Option<Vec<f32>> {
        match self {
            Self::F32(image) => Some(image.into_raw()),
            Self::U8(_) | Self::U16(_) | Self::U32(_) => None,
            _ => None,
        }
    }
}

/// An error produced while constructing a [`SampleBuffer`].
///
/// # Examples
///
/// ```
/// use fornax_codec::SampleBuffer;
///
/// let error = SampleBuffer::from_u8(2, 2, vec![1, 2, 3]).unwrap_err();
/// assert!(error.to_string().contains("requires 4"));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum SampleBufferError {
    /// The sample count for the requested dimensions does not fit `usize`.
    #[error("sample count for {width}x{height} overflows usize")]
    SampleCountOverflow {
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
    },
    /// The number of provided samples does not match the dimensions.
    #[error(
        "sample buffer has {actual} samples; {width}x{height} requires {expected} samples"
    )]
    SampleCountMismatch {
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
        /// Number of samples required by the dimensions.
        expected: usize,
        /// Number of samples provided.
        actual: usize,
    },
    /// The requested dimensions cannot form an image buffer.
    #[error("image dimensions {width}x{height} are invalid")]
    InvalidDimensions {
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
    },
}

fn create<T>(width: u32, height: u32, data: Vec<T>) -> Result<Mono<T>, SampleBufferError>
where
    T: image::Primitive,
{
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(SampleBufferError::SampleCountOverflow { width, height })?;
    let actual = data.len();
    if actual != expected {
        return Err(SampleBufferError::SampleCountMismatch {
            width,
            height,
            expected,
            actual,
        });
    }
    ImageBuffer::from_raw(width, height, data)
        .ok_or(SampleBufferError::InvalidDimensions { width, height })
}
