use core::slice;
use std::fmt::Display;
#[derive(Debug)]
pub enum ImageFormats {
    LibrawImageJpeg = 1,
    ImageBitmap = 2,
}
impl Display for ImageFormats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormats::ImageBitmap => write!(f, "ImageBitmap"),
            ImageFormats::LibrawImageJpeg => write!(f, "LibrawImageJpeg"),
        }
    }
}
pub struct DcRawProcessedImage {
    processed_image: *mut libraw_sys::libraw_processed_image_t,
}
impl DcRawProcessedImage {
    pub(crate) fn new(
        ptr: *mut libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<DcRawProcessedImage> {
        if ptr.is_null() {
            miette::bail!("`libraw_processed_image_t` pointer is null.")
        }
        clerk::debug!("{:?}", unsafe { *(ptr) });
        let img: DcRawProcessedImage = Self {
            processed_image: ptr,
        };
        Ok(img)
    }
    /// This field records type of data, containing in remaining fields of structure.
    /// - LIBRAW_IMAGE_BITMAP - structure contains RGB bitmap. All metadata fields (see below) are
    ///   valid and describes image data.
    /// - LIBRAW_IMAGE_JPEG - structure contain in-memory image of JPEG file. Only type, data_size
    ///   and data fields are valid (and nonzero);
    pub fn image_type(&self) -> miette::Result<ImageFormats> {
        match unsafe { (*self.processed_image).type_ } {
            1i32 => Ok(ImageFormats::LibrawImageJpeg),
            2i32 => Ok(ImageFormats::ImageBitmap),
            t => miette::bail!("Unknow image format: {t}"),
        }
    }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn height(&self) -> u16 {
        unsafe { (*self.processed_image).height }
    }
    /// Image size (in pixels). Valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn width(&self) -> u16 {
        unsafe { (*self.processed_image).width }
    }
    /// Number of colors components (1 or 3) and color depth in bits (8 or 16). These fields are
    /// valid only if type==LIBRAW_IMAGE_BITMAP.
    pub fn colors(&self) -> u16 {
        unsafe { (*self.processed_image).colors }
    }
    ///Is bitmap data gamma-corrected (always 1 for 8-bit data, may be 0 or 1 for 16-bit). Valid
    /// only if type==LIBRAW_IMAGE_BITMAP.
    pub fn bits(&self) -> u16 {
        unsafe { (*self.processed_image).bits }
    }
    ///Size of data field (in bytes). For bitmap image equal to (height*width*colors * (bits/8)).
    /// For JPEG image - exact JPEG size (i.e. extracted thnumbnail size + JPEG header + EXIF
    /// header).
    pub fn data_size(&self) -> u32 {
        unsafe { (*self.processed_image).data_size }
    }
    ///Size of data field (in bytes). For bitmap image equal to (height*width*colors * (bits/8)).
    /// For JPEG image - exact JPEG size (i.e. extracted thnumbnail size + JPEG header + EXIF
    /// header).
    pub fn data(&self) -> *const u8 {
        unsafe { (*self.processed_image).data.as_ptr() }
    }
}
impl Drop for DcRawProcessedImage {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_dcraw_clear_mem(self.processed_image) }
    }
}

impl DcRawProcessedImage {
    pub fn to_image(&self) -> miette::Result<fornax_core::ProcessedImage> {
        clerk::debug!("Start cast to image.");
        match (self.colors(), self.bits()) {
            (1, 8) => {
                let img: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                            .to_vec()
                        },
                    )
                    .unwrap();
                clerk::debug!("Finish cast to image.");
                Ok(fornax_core::ProcessedImage::Mono8(img))
            }
            (1, 16) => {
                let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        bytemuck::cast_slice::<u8, u16>(unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                        })
                        .to_vec(),
                    )
                    .unwrap();
                clerk::debug!("Finish cast to image.");
                Ok(fornax_core::ProcessedImage::Mono16(img))
            }
            (3, 8) => {
                let img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                            .to_vec()
                        },
                    )
                    .unwrap();
                clerk::debug!("Finish cast to image.");
                Ok(fornax_core::ProcessedImage::Rgb8(img))
            }
            (3, 16) => {
                let img: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        bytemuck::cast_slice::<u8, u16>(unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                        })
                        .to_vec(),
                    )
                    .unwrap();
                clerk::debug!("Finish cast to image.");
                Ok(fornax_core::ProcessedImage::Rgb16(img))
            }
            (c, b) => {
                miette::bail!("Unsupported color:{}, bits: {}.", c, b)
            }
        }
    }
}
