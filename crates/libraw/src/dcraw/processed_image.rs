use miette::IntoDiagnostic;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(i32)]
pub enum DCRawImageFormats {
    Jpeg = libraw_sys::LibRaw_image_formats_LIBRAW_IMAGE_JPEG as i32,
    Bitmap = libraw_sys::LibRaw_image_formats_LIBRAW_IMAGE_BITMAP as i32,
}

pub struct DCRawProcessedImage {
    processed_image: *mut libraw_sys::libraw_processed_image_t,
}
impl DCRawProcessedImage {
    pub(crate) fn new(
        ptr: *mut libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<DCRawProcessedImage> {
        if ptr.is_null() {
            miette::bail!("`libraw_processed_image_t` pointer is null.")
        }
        clerk::debug!("{:?}", unsafe { *(ptr) });
        let img: DCRawProcessedImage = Self {
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
    pub fn image_type(&self) -> miette::Result<DCRawImageFormats> {
        DCRawImageFormats::try_from(unsafe { (*self.processed_image).type_ }).into_diagnostic()
    }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn height(&self) -> u16 { unsafe { (*self.processed_image).height } }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn width(&self) -> u16 { unsafe { (*self.processed_image).width } }
    /// Number of colors components (1 or 3) and color depth in bits (8 or 16).
    /// These fields are valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn colors(&self) -> u16 { unsafe { (*self.processed_image).colors } }
    ///Is bitmap data gamma-corrected (always 1 for 8-bit data, may be 0 or 1
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
}
impl Drop for DCRawProcessedImage {
    fn drop(&mut self) { unsafe { libraw_sys::libraw_dcraw_clear_mem(self.processed_image) } }
}
