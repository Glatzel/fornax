#![allow(
    dead_code,
    reason = "native backend support is shared by production decoding and backend-specific examples"
)]

use std::ffi::c_char;
use std::marker::PhantomData;
use std::path::Path;

use envoy::{PtrToString, ToCString};
use image::ImageBuffer;

#[derive(Clone, Copy, Debug, Eq, PartialEq, num_enum::FromPrimitive)]
#[repr(i32)]
pub enum LibRawErrorCode {
    Other = 2,
    #[num_enum(default)]
    Unknown = 1,
    Success = libraw_sys::LibRaw_errors_LIBRAW_SUCCESS,
    Unspecified = libraw_sys::LibRaw_errors_LIBRAW_UNSPECIFIED_ERROR,
    FileUnsupported = libraw_sys::LibRaw_errors_LIBRAW_FILE_UNSUPPORTED,
    RequestForNonexistentImage = libraw_sys::LibRaw_errors_LIBRAW_REQUEST_FOR_NONEXISTENT_IMAGE,
    OutOfOrder = libraw_sys::LibRaw_errors_LIBRAW_OUT_OF_ORDER_CALL,
    NoThumbnail = libraw_sys::LibRaw_errors_LIBRAW_NO_THUMBNAIL,
    UnsupportedThumbnail = libraw_sys::LibRaw_errors_LIBRAW_UNSUPPORTED_THUMBNAIL,
    InputClosed = libraw_sys::LibRaw_errors_LIBRAW_INPUT_CLOSED,
    NotImplemented = libraw_sys::LibRaw_errors_LIBRAW_NOT_IMPLEMENTED,
    RequestForNonexistentThumbnail =
        libraw_sys::LibRaw_errors_LIBRAW_REQUEST_FOR_NONEXISTENT_THUMBNAIL,
    InsufficientMemory = libraw_sys::LibRaw_errors_LIBRAW_UNSUFFICIENT_MEMORY,
    DataError = libraw_sys::LibRaw_errors_LIBRAW_DATA_ERROR,
    IoError = libraw_sys::LibRaw_errors_LIBRAW_IO_ERROR,
    CancelledByCallback = libraw_sys::LibRaw_errors_LIBRAW_CANCELLED_BY_CALLBACK,
    BadCrop = libraw_sys::LibRaw_errors_LIBRAW_BAD_CROP,
    TooBig = libraw_sys::LibRaw_errors_LIBRAW_TOO_BIG,
    MempoolOverflow = libraw_sys::LibRaw_errors_LIBRAW_MEMPOOL_OVERFLOW,
}

#[derive(Debug, thiserror::Error)]
pub enum LibRawError {
    #[error("invalid LibRaw input: {0}")]
    InvalidInput(String),
    #[error("LibRaw error {code:?}: {message}")]
    Native {
        code: LibRawErrorCode,
        message: String,
    },
}

impl LibRawError {
    fn native(raw: i32) -> Self {
        let code = LibRawErrorCode::from(raw);
        let message = unsafe {
            let pointer = libraw_sys::libraw_strerror(raw);
            if pointer.is_null() {
                format!("LibRaw error {raw}")
            } else {
                pointer
                    .to_string_lossy()
                    .unwrap_or_else(|_| format!("LibRaw error {raw}"))
            }
        };
        Self::Native { code, message }
    }
}

pub type LibRawResult<T> = Result<T, LibRawError>;

pub struct LibRawContext {
    pointer: *mut libraw_sys::libraw_data_t,
    _buffer_owner: Option<Vec<u8>>,
}

impl LibRawContext {
    pub fn new() -> LibRawResult<Self> {
        let pointer = unsafe { libraw_sys::libraw_init(0) };
        if pointer.is_null() {
            return Err(LibRawError::Native {
                code: LibRawErrorCode::InsufficientMemory,
                message: "LibRaw initialization returned a null context".into(),
            });
        }
        Ok(Self {
            pointer,
            _buffer_owner: None,
        })
    }

    pub fn open_file(&mut self, path: &Path) -> LibRawResult<()> {
        let path = path
            .to_str()
            .ok_or_else(|| LibRawError::InvalidInput("path is not valid UTF-8".into()))?;
        let path = path
            .to_cstring()
            .map_err(|error| LibRawError::InvalidInput(error.to_string()))?;
        check(unsafe { libraw_sys::libraw_open_file(self.pointer, path.as_ptr()) })
    }

    pub fn open_buffer(&mut self, input: &[u8]) -> LibRawResult<()> {
        let buffer = input.to_vec();
        check(unsafe {
            libraw_sys::libraw_open_buffer(self.pointer, buffer.as_ptr().cast(), buffer.len())
        })?;
        self._buffer_owner = Some(buffer);
        Ok(())
    }

    pub fn unpack(&mut self) -> LibRawResult<()> {
        check(unsafe { libraw_sys::libraw_unpack(self.pointer) })
    }

    pub fn image_sizes(&self) -> ImageSizes {
        let sizes = unsafe { (*self.pointer).sizes };
        ImageSizes {
            raw_width: u32::from(sizes.raw_width),
            raw_height: u32::from(sizes.raw_height),
            width: u32::from(sizes.width),
            height: u32::from(sizes.height),
            top_margin: u32::from(sizes.top_margin),
            left_margin: u32::from(sizes.left_margin),
            raw_pitch: sizes.raw_pitch,
            flip: sizes.flip,
        }
    }

    pub fn parameters(&self) -> LibRawResult<ImageParameters> {
        let data = unsafe { &(*self.pointer).idata };
        Ok(ImageParameters {
            make: c_array_string(&data.make)?,
            model: c_array_string(&data.model)?,
            software: c_array_string(&data.software)?,
            normalized_make: c_array_string(&data.normalized_make)?,
            normalized_model: c_array_string(&data.normalized_model)?,
            maker_index: data.maker_index,
            cdesc: c_array_string(&data.cdesc)?,
            raw_count: data.raw_count,
            dng_version: data.dng_version,
            is_foveon: data.is_foveon != 0,
            colors: data.colors,
            filters: data.filters,
        })
    }

    pub fn color_data(&self) -> ColorData {
        let data = unsafe { &(*self.pointer).color };
        let cblack = data.cblack;
        ColorData {
            black: data.black,
            cblack: [
                cblack[0], cblack[1], cblack[2], cblack[3], cblack[4], cblack[5],
            ],
            maximum: data.maximum,
            linear_max: data.linear_max,
            dng_whitelevel: data.dng_levels.dng_whitelevel,
            default_crop: data.dng_levels.default_crop,
            cam_mul: data.cam_mul,
            rgb_cam: data.rgb_cam,
            cam_xyz: data.cam_xyz,
            cmatrix: data.cmatrix,
            ccm: data.ccm,
            raw_bps: data.raw_bps,
            data_maximum: data.data_maximum,
        }
    }

    pub fn capture_data(&self) -> CaptureData {
        let data = unsafe { &(*self.pointer).other };
        CaptureData {
            timestamp: Some(data.timestamp),
            iso_speed: data.iso_speed,
            shutter: data.shutter,
            aperture: data.aperture,
            focal_length: data.focal_len,
        }
    }

    pub fn unpacked_raw_image(&self) -> LibRawResult<UnpackedRawImage<'_>> {
        check_raw_alloc(self.pointer)?;
        let data = unsafe { &(*self.pointer) };
        Ok(UnpackedRawImage {
            integer: data.rawdata.raw_image.cast_const(),
            float: data.rawdata.float_image.cast_const(),
            raw_width: u32::from(data.sizes.raw_width),
            raw_height: u32::from(data.sizes.raw_height),
            raw_pitch: data.sizes.raw_pitch,
            has_color3_image: !data.rawdata.color3_image.is_null(),
            has_color4_image: !data.rawdata.color4_image.is_null(),
            has_float3_image: !data.rawdata.float3_image.is_null(),
            has_float4_image: !data.rawdata.float4_image.is_null(),
            _owner: PhantomData,
        })
    }

    pub fn dcraw_process_neutral(&mut self) -> LibRawResult<()> {
        let params = unsafe { &mut (*self.pointer).params };
        params.gamm[0] = 1.0;
        params.gamm[1] = 1.0;
        params.user_mul = [1.0; 4];
        params.highlight = 0;
        params.use_auto_wb = 0;
        params.use_camera_wb = 0;
        params.use_camera_matrix = 0;
        params.output_color = 0;
        params.output_bps = 16;
        params.user_flip = 0;
        params.user_qual = 0;
        params.no_auto_bright = 1;
        params.fbdd_noiserd = 0;
        params.no_auto_scale = 1;
        params.no_interpolation = 0;
        check(unsafe { libraw_sys::libraw_dcraw_process(self.pointer) })
    }

    pub fn make_processed_image(&self) -> LibRawResult<ProcessedImage> {
        let mut error = 0;
        let pointer =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.pointer, &raw mut error) };
        check(error)?;
        if pointer.is_null() {
            return Err(LibRawError::InvalidInput(
                "LibRaw returned a null processed image".into(),
            ));
        }
        Ok(ProcessedImage { pointer })
    }

    pub fn progress_flags(&self) -> u32 { unsafe { (*self.pointer).progress_flags } }

    pub fn unpack_function_name(&self) -> Option<String> {
        unsafe {
            let name = libraw_sys::libraw_unpack_function_name(self.pointer);
            (!name.is_null())
                .then(|| name.to_string_lossy().ok())
                .flatten()
        }
    }
}

impl Drop for LibRawContext {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            unsafe { libraw_sys::libraw_close(self.pointer) }
        }
    }
}

pub fn version() -> Option<String> {
    unsafe {
        let pointer = libraw_sys::libraw_version();
        (!pointer.is_null())
            .then(|| pointer.to_string_lossy().ok())
            .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct ImageSizes {
    pub raw_width: u32,
    pub raw_height: u32,
    pub width: u32,
    pub height: u32,
    pub top_margin: u32,
    pub left_margin: u32,
    pub raw_pitch: u32,
    pub flip: i32,
}

#[derive(Clone, Debug)]
pub struct ImageParameters {
    pub make: String,
    pub model: String,
    pub software: String,
    pub normalized_make: String,
    pub normalized_model: String,
    pub maker_index: u32,
    pub cdesc: String,
    pub raw_count: u32,
    pub dng_version: u32,
    pub is_foveon: bool,
    pub colors: i32,
    pub filters: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CaptureData {
    pub timestamp: Option<i64>,
    pub iso_speed: f32,
    pub shutter: f32,
    pub aperture: f32,
    pub focal_length: f32,
}

#[derive(Clone, Debug)]
pub struct ColorData {
    pub black: u32,
    pub cblack: [u32; 6],
    pub maximum: u32,
    pub linear_max: [u32; 4],
    pub dng_whitelevel: [u32; 4],
    pub default_crop: [u16; 4],
    pub cam_mul: [f32; 4],
    pub rgb_cam: [[f32; 4]; 3],
    pub cam_xyz: [[f32; 3]; 4],
    pub cmatrix: [[f32; 4]; 3],
    pub ccm: [[f32; 4]; 3],
    pub raw_bps: u32,
    pub data_maximum: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnpackedRawImageKind {
    Integer,
    Float,
    None,
}

#[derive(Debug)]
pub struct UnpackedRawImage<'a> {
    integer: *const u16,
    float: *const f32,
    raw_width: u32,
    raw_height: u32,
    raw_pitch: u32,
    has_color3_image: bool,
    has_color4_image: bool,
    has_float3_image: bool,
    has_float4_image: bool,
    _owner: PhantomData<&'a LibRawContext>,
}

impl UnpackedRawImage<'_> {
    pub const fn raw_width(&self) -> u32 { self.raw_width }

    pub const fn raw_height(&self) -> u32 { self.raw_height }

    pub const fn kind(&self) -> UnpackedRawImageKind {
        if !self.integer.is_null() {
            UnpackedRawImageKind::Integer
        } else if !self.float.is_null() {
            UnpackedRawImageKind::Float
        } else {
            UnpackedRawImageKind::None
        }
    }

    pub fn integer_row(&self, row: u32) -> LibRawResult<Option<&[u16]>> {
        if self.integer.is_null() {
            return Ok(None);
        }
        let stride = self.validate_row(row, size_of::<u16>())?;
        Ok(Some(unsafe {
            std::slice::from_raw_parts(
                self.integer.add(row as usize * stride),
                self.raw_width as usize,
            )
        }))
    }

    pub fn float_row(&self, row: u32) -> LibRawResult<Option<&[f32]>> {
        if self.float.is_null() {
            return Ok(None);
        }
        let stride = self.validate_row(row, size_of::<f32>())?;
        Ok(Some(unsafe {
            std::slice::from_raw_parts(
                self.float.add(row as usize * stride),
                self.raw_width as usize,
            )
        }))
    }

    pub const fn has_color3_image(&self) -> bool { self.has_color3_image }

    pub const fn has_color4_image(&self) -> bool { self.has_color4_image }

    pub const fn has_float3_image(&self) -> bool { self.has_float3_image }

    pub const fn has_float4_image(&self) -> bool { self.has_float4_image }

    fn validate_row(&self, row: u32, sample_size: usize) -> LibRawResult<usize> {
        let row_bytes = self
            .raw_width
            .checked_mul(sample_size as u32)
            .ok_or_else(|| LibRawError::InvalidInput("raw row size overflows".into()))?;
        if row >= self.raw_height
            || self.raw_pitch < row_bytes
            || !self.raw_pitch.is_multiple_of(sample_size as u32)
        {
            return Err(LibRawError::InvalidInput(
                "unpacked raw-image geometry is inconsistent".into(),
            ));
        }
        usize::try_from(self.raw_pitch / sample_size as u32)
            .map_err(|_| LibRawError::InvalidInput("raw row stride overflows usize".into()))
    }
}

#[derive(Debug)]
pub struct ProcessedImage {
    pointer: *mut libraw_sys::libraw_processed_image_t,
}

impl ProcessedImage {
    pub fn rgb16_image(&self) -> LibRawResult<ImageBuffer<image::Rgb<u16>, Vec<u16>>> {
        let data = unsafe { &*self.pointer };
        if data.colors != 3 || data.bits != 16 {
            return Err(LibRawError::InvalidInput(
                "processed image is not a three-channel 16-bit bitmap".into(),
            ));
        }
        let sample_count = usize::from(data.width)
            .checked_mul(usize::from(data.height))
            .and_then(|count| count.checked_mul(3))
            .ok_or_else(|| LibRawError::InvalidInput("processed dimensions overflow".into()))?;
        let byte_count = sample_count
            .checked_mul(2)
            .ok_or_else(|| LibRawError::InvalidInput("processed byte count overflows".into()))?;
        let bytes =
            unsafe { std::slice::from_raw_parts(data.data.as_ptr(), data.data_size as usize) };
        if bytes.len() != byte_count {
            return Err(LibRawError::InvalidInput(
                "processed data length is inconsistent".into(),
            ));
        }
        let samples = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|sample| u16::from_ne_bytes([sample[0], sample[1]]))
            .collect();
        ImageBuffer::from_raw(u32::from(data.width), u32::from(data.height), samples)
            .ok_or_else(|| LibRawError::InvalidInput("processed image is inconsistent".into()))
    }
}

impl Drop for ProcessedImage {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            unsafe { libraw_sys::libraw_dcraw_clear_mem(self.pointer) }
        }
    }
}

fn check(code: i32) -> LibRawResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(LibRawError::native(code))
    }
}

fn check_raw_alloc(pointer: *mut libraw_sys::libraw_data_t) -> LibRawResult<()> {
    if pointer.is_null() || unsafe { (*pointer).rawdata.raw_alloc.is_null() } {
        Err(LibRawError::InvalidInput(
            "LibRaw raw allocation is unavailable".into(),
        ))
    } else {
        Ok(())
    }
}

fn c_array_string<const N: usize>(value: &[c_char; N]) -> LibRawResult<String> {
    value
        .as_slice()
        .to_string_lossy()
        .map_err(|error| LibRawError::InvalidInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_discriminants_use_libraw_sys_values() {
        assert_eq!(
            LibRawErrorCode::FileUnsupported as i32,
            libraw_sys::LibRaw_errors_LIBRAW_FILE_UNSUPPORTED
        );
        assert_eq!(
            LibRawErrorCode::IoError as i32,
            libraw_sys::LibRaw_errors_LIBRAW_IO_ERROR
        );
        assert_eq!(
            LibRawErrorCode::MempoolOverflow as i32,
            libraw_sys::LibRaw_errors_LIBRAW_MEMPOOL_OVERFLOW
        );
    }
}
