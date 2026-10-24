use image::{ImageBuffer, Rgb};
use num_enum::TryFromPrimitive;

use crate::LibrawError;
use crate::extension::error_handling::check_run;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(i32)]
pub enum DCRawImageFormats {
    Jpeg = 1,
    Bitmap = 2,
}
#[derive(Debug)]
pub struct ProcessedImage {
    processed_image: *mut libraw_sys::libraw_processed_image_t,
}
impl ProcessedImage {
    pub(crate) fn new(ptr: *mut libraw_sys::libraw_processed_image_t) -> Result<Self, LibrawError> {
        check_run!(ptr.is_null(), "`libraw_processed_image_t` pointer is null.");
        clerk::debug!("{:?}", unsafe { *(ptr) });
        let img: Self = Self {
            processed_image: ptr,
        };
        Ok(img)
    }
    /// This field records type of data, containing in remaining fields of
    /// structure.
    /// - LIBRAW_IMAGE_BITMAP - structure contains RGB bitmap. All metadata
    ///   fields (see below) are valid and describes image data.
    /// - LIBRAW_IMAGE_JPEG - structure contain in-memory image of JPEG file.
    ///   Only type, data_size and data fields are valid (and nonzero);
    #[allow(clippy::unnecessary_cast, reason = "libc use usize on unix.")]
    pub fn image_type(&self) -> Result<DCRawImageFormats, LibrawError> {
        Ok(DCRawImageFormats::try_from(unsafe {
            (*self.processed_image).type_ as i32
        })?)
    }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn height(&self) -> u16 { unsafe { (*self.processed_image).height } }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn width(&self) -> u16 { unsafe { (*self.processed_image).width } }
    /// Number of colors components (1 or 3) and color depth in bits (8 or 16).
    /// These fields are valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn colors(&self) -> u16 { unsafe { (*self.processed_image).colors } }
    ///Is bitmap data gamma-correctted (always 1 for 8-bit data, may be 0 or 1
    /// for 16-bit). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn bits(&self) -> u16 { unsafe { (*self.processed_image).bits } }
    ///Size of data field (in bytes). For bitmap image equal to
    /// (height*width*colors * (bits/8)). For JPEG image - exact JPEG size
    /// (i.e. extracted thnumbnail size + JPEG header + EXIF header).
    pub fn data_size(&self) -> u32 { unsafe { (*self.processed_image).data_size } }
    ///Size of data field (in bytes). For bitmap image equal to
    /// (height*width*colors * (bits/8)). For JPEG image - exact JPEG size
    /// (i.e. extracted thnumbnail size + JPEG header + EXIF header).
    pub fn data(&self) -> *const u8 { unsafe { (*self.processed_image).data.as_ptr() } }

    /// Copies a three-channel 16-bit bitmap into an image-rs buffer.
    ///
    /// # Errors
    ///
    /// Returns an error unless this is a three-color 16-bit bitmap with
    /// dimensions matching its data length.
    pub fn rgb16_image(&self) -> Result<ImageBuffer<Rgb<u16>, Vec<u16>>, LibrawError> {
        check_run!(
            self.colors() != 3 || self.bits() != 16,
            "processed image is not a three-channel 16-bit bitmap"
        );
        let sample_count = usize::from(self.width())
            .checked_mul(usize::from(self.height()))
            .and_then(|count| count.checked_mul(3))
            .ok_or_else(|| invalid_processed_image("processed image dimensions overflow usize"))?;
        let byte_count = sample_count
            .checked_mul(2)
            .ok_or_else(|| invalid_processed_image("processed image byte count overflows usize"))?;
        // SAFETY: LibRaw owns `data_size` bytes for this processed image.
        let bytes = unsafe { std::slice::from_raw_parts(self.data(), self.data_size() as usize) };
        if bytes.len() != byte_count {
            return Err(invalid_processed_image(
                "processed image data size does not match its dimensions",
            ));
        }
        let samples = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|sample| u16::from_ne_bytes([sample[0], sample[1]]))
            .collect();
        ImageBuffer::from_raw(u32::from(self.width()), u32::from(self.height()), samples)
            .ok_or_else(|| {
                invalid_processed_image("processed image dimensions do not match its sample count")
            })
    }
}

fn invalid_processed_image(message: &str) -> LibrawError {
    LibrawError::Libraw {
        code: crate::LibrawErrorCode::DataError,
        message: message.into(),
    }
}
impl Drop for ProcessedImage {
    fn drop(&mut self) { unsafe { libraw_sys::libraw_dcraw_clear_mem(self.processed_image) } }
}
