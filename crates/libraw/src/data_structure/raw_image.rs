use std::ffi::CStr;
use std::marker::PhantomData;

use crate::extension::error_handling::check_raw_alloc;
use crate::{Libraw, LibrawError, LibrawErrorCode};

/// The native single-plane sample representation returned by LibRaw.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnpackedRawImageKind {
    /// Unsigned 16-bit integer samples.
    Integer,
    /// 32-bit floating-point samples.
    Float,
    /// No single-plane unpacked image is present.
    None,
}

/// A borrowed, read-only view of LibRaw's unpacked raw-image fields.
#[derive(Debug)]
pub struct UnpackedRawImage<'a> {
    integer: *const u16,
    float: *const f32,
    raw_width: u16,
    raw_height: u16,
    raw_pitch: u32,
    has_color3_image: bool,
    has_color4_image: bool,
    has_float3_image: bool,
    has_float4_image: bool,
    _owner: PhantomData<&'a Libraw>,
}

impl Libraw {
    /// Returns a borrowed view of the unpacked raw-image pointers.
    ///
    /// # Errors
    ///
    /// Returns an error when LibRaw has not allocated a raw image.
    pub fn get_unpacked_raw_image(&self) -> Result<UnpackedRawImage<'_>, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        // SAFETY: `raw_alloc` was checked above and the returned view borrows
        // this context for its full lifetime.
        let data = unsafe { &*self.imgdata_ptr() };
        Ok(UnpackedRawImage {
            integer: data.rawdata.raw_image.cast_const(),
            float: data.rawdata.float_image.cast_const(),
            raw_width: data.sizes.raw_width,
            raw_height: data.sizes.raw_height,
            raw_pitch: data.sizes.raw_pitch,
            has_color3_image: !data.rawdata.color3_image.is_null(),
            has_color4_image: !data.rawdata.color4_image.is_null(),
            has_float3_image: !data.rawdata.float3_image.is_null(),
            has_float4_image: !data.rawdata.float4_image.is_null(),
            _owner: PhantomData,
        })
    }

    /// Returns the current LibRaw progress flags.
    pub fn progress_flags(&self) -> u32 { unsafe { (*self.imgdata_ptr()).progress_flags } }

    /// Returns the active unpack function name reported by LibRaw.
    pub fn unpack_function_name(&self) -> Option<String> {
        // SAFETY: LibRaw documents the returned pointer as a valid,
        // null-terminated function name for the lifetime of the context.
        unsafe {
            let name = libraw_sys::libraw_unpack_function_name(self.imgdata_ptr());
            (!name.is_null()).then(|| CStr::from_ptr(name).to_string_lossy().into_owned())
        }
    }
}

impl UnpackedRawImage<'_> {
    /// Returns the full stored raster width.
    pub const fn raw_width(&self) -> u16 { self.raw_width }

    /// Returns the full stored raster height.
    pub const fn raw_height(&self) -> u16 { self.raw_height }

    /// Returns the full stored raster row stride in bytes.
    pub const fn raw_pitch(&self) -> u32 { self.raw_pitch }

    /// Returns the single-plane sample representation.
    pub const fn kind(&self) -> UnpackedRawImageKind {
        if !self.integer.is_null() {
            UnpackedRawImageKind::Integer
        } else if !self.float.is_null() {
            UnpackedRawImageKind::Float
        } else {
            UnpackedRawImageKind::None
        }
    }

    /// Returns one unsigned integer row.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range row, an absent integer plane, or
    /// an incompatible row stride.
    pub fn integer_row(&self, row: u32) -> Result<Option<&[u16]>, LibrawError> {
        if self.integer.is_null() {
            return Ok(None);
        }
        let stride = self.validate_row(row, size_of::<u16>())?;
        // SAFETY: the stride and width were checked against the reported
        // allocation geometry and the view borrows the owning LibRaw context.
        Ok(Some(unsafe {
            std::slice::from_raw_parts(
                self.integer.add(row as usize * stride),
                self.raw_width.into(),
            )
        }))
    }

    /// Returns one floating-point row.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range row, an absent float plane, or an
    /// incompatible row stride.
    pub fn float_row(&self, row: u32) -> Result<Option<&[f32]>, LibrawError> {
        if self.float.is_null() {
            return Ok(None);
        }
        let stride = self.validate_row(row, size_of::<f32>())?;
        // SAFETY: the stride and width were checked against the reported
        // allocation geometry and the view borrows the owning LibRaw context.
        Ok(Some(unsafe {
            std::slice::from_raw_parts(self.float.add(row as usize * stride), self.raw_width.into())
        }))
    }

    /// Returns whether LibRaw exposes a three-channel integer image.
    pub const fn has_color3_image(&self) -> bool { self.has_color3_image }

    /// Returns whether LibRaw exposes a four-channel integer image.
    pub const fn has_color4_image(&self) -> bool { self.has_color4_image }

    /// Returns whether LibRaw exposes a three-channel float image.
    pub const fn has_float3_image(&self) -> bool { self.has_float3_image }

    /// Returns whether LibRaw exposes a four-channel float image.
    pub const fn has_float4_image(&self) -> bool { self.has_float4_image }

    fn validate_row(&self, row: u32, sample_size: usize) -> Result<usize, LibrawError> {
        let row_bytes = u32::from(self.raw_width)
            .checked_mul(sample_size as u32)
            .ok_or_else(geometry_error)?;
        if row >= u32::from(self.raw_height)
            || self.raw_pitch < row_bytes
            || !self.raw_pitch.is_multiple_of(sample_size as u32)
        {
            return Err(geometry_error());
        }
        usize::try_from(self.raw_pitch / sample_size as u32).map_err(|_| geometry_error())
    }
}

fn geometry_error() -> LibrawError {
    LibrawError::Libraw {
        code: LibrawErrorCode::DataError,
        message: "unpacked raw-image geometry is inconsistent".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_unpacked_api_reads_the_cr2_fixture() -> mischief::Result<()> {
        let context = Libraw::default();
        context.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let sizes = context.get_image_sizes()?;
        let params = context.get_iparams()?;
        let color = context.get_color_data()?;
        let raw = context.get_unpacked_raw_image()?;

        assert_eq!(sizes.raw_width(), 5_360);
        assert_eq!(sizes.raw_height(), 3_516);
        assert_eq!(sizes.raw_pitch(), 10_720);
        assert_eq!(params.make()?, "Canon");
        assert_eq!(params.model()?, "EOS 7D");
        assert_eq!(params.cdesc_text()?, "RGBG");
        assert!(color.raw_bps() > 0);
        assert_eq!(color.black(), 2_048);
        assert_eq!(&color.cblack()[..4], &[0, 0, 0, 0]);
        assert_eq!(&color.cblack()[4..], &[0, 0]);
        assert_eq!(raw.kind(), UnpackedRawImageKind::Integer);
        assert_eq!(
            raw.integer_row(0)?.unwrap().len(),
            usize::from(raw.raw_width())
        );
        assert_eq!(context.progress_flags() & (1 << 4), 0);
        Ok(())
    }
}
