mod image_sizes;
mod imgother;
mod iparams;
mod rawdata;
use std::path::Path;
use std::{ffi::CString, slice};

use crate::ILibrawErrors;
use fornax_core::{FornaxRawImage, IDecoder};
use image::ImageBuffer;
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::LibrawIParams;
pub use rawdata::LibrawRawdata;
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
}

impl Libraw {
    pub fn new() -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata }
    }

    // io
    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(
            unsafe {
                libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
            },
            "libraw_open_buffer",
        )?;
        Ok(())
    }

    pub fn open_file(&self, fname: &Path) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(
            unsafe { libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _) },
            "libraw_open_file",
        )?;
        Ok(())
    }

    pub fn unpack(&self) -> miette::Result<()> {
        Self::check_run(
            unsafe { libraw_sys::libraw_unpack(self.imgdata) },
            "libraw_unpack",
        )?;
        Ok(())
    }

    // data structure
    pub fn imgother(&self) -> miette::Result<LibrawImgOther> {
        LibrawImgOther::new(self.imgdata)
    }
    pub fn image_sizes(&self) -> miette::Result<LibrawImageSizes> {
        LibrawImageSizes::new(self.imgdata)
    }
    pub fn iparams(&self) -> miette::Result<LibrawIParams> {
        LibrawIParams::new(self.imgdata)
    }
    pub fn rawdata(&self, raw_image_type: &LibrawRawdata) -> miette::Result<FornaxRawImage> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        let size = self.image_sizes()?;
        let width: u32 = size.raw_width() as u32;
        let height: u32 = size.raw_height() as u32;
        match raw_image_type {
            LibrawRawdata::RawImage => {
                let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> = {
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.raw_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                    })
                    .unwrap()
                };
                Ok(FornaxRawImage::Mono16(img))
            }
            LibrawRawdata::Color3Image => {
                let img: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.color3_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                        .into_iter()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<u16>>()
                    })
                    .unwrap();
                Ok(FornaxRawImage::Rgb16(img))
            }
            LibrawRawdata::Color4Image => {
                let img: image::ImageBuffer<image::Rgba<u16>, Vec<u16>> =
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.color4_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                        .into_iter()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<u16>>()
                    })
                    .unwrap();
                Ok(FornaxRawImage::Rgba16(img))
            }
            LibrawRawdata::FloatImage => {
                let img: image::ImageBuffer<image::Luma<f32>, Vec<f32>> = {
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.float_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                    })
                    .unwrap()
                };
                Ok(FornaxRawImage::MonoF32(img))
            }
            LibrawRawdata::Float3Image => {
                let img: image::ImageBuffer<image::Rgb<f32>, Vec<f32>> =
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.float3_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                        .into_iter()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<f32>>()
                    })
                    .unwrap();
                Ok(FornaxRawImage::RgbF32(img))
            }
            LibrawRawdata::Float4Image => {
                let img: image::ImageBuffer<image::Rgba<f32>, Vec<f32>> =
                    ImageBuffer::from_vec(width, height, unsafe {
                        slice::from_raw_parts(
                            (*self.imgdata).rawdata.float4_image,
                            width as usize * height as usize,
                        )
                        .to_vec()
                        .into_iter()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<f32>>()
                    })
                    .unwrap();
                Ok(FornaxRawImage::RgbaF32(img))
            }
        }
    }
}
impl Drop for Libraw {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}
impl Default for Libraw {
    fn default() -> Self {
        Self::new()
    }
}
impl crate::IDCRaw for Libraw {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t> {
        Ok(self.imgdata)
    }
}
impl IDecoder for Libraw {
    fn decode_file(&self, file: &Path) -> miette::Result<()> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
}
impl ILibrawErrors for Libraw {}
