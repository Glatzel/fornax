use core::slice;

use fornax_core::FornaxRawImage;
use image::ImageBuffer;

pub struct LibrawRawdata {}
impl LibrawRawdata {
    pub(crate) fn get_rawdata(
        imgdata: *mut libraw_sys::libraw_data_t,
        width: usize,
        height: usize,
    ) -> miette::Result<FornaxRawImage> {
        if unsafe { (*imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        unsafe { libraw_sys::libraw_raw2image(imgdata) };
        unsafe { libraw_sys::libraw_subtract_black(imgdata) };
        if !unsafe { (*imgdata).rawdata.raw_image }.is_null() {
            clerk::debug!("Found mono16 raw image.");
            let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> = {
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.raw_image, width * height).to_vec()
                })
                .unwrap()
            };
            Ok(FornaxRawImage::Mono16(img))
        } else if !unsafe { (*imgdata).image }.is_null() {
            clerk::debug!("Found rgba16 raw image.");
            let img: image::ImageBuffer<image::Rgba<u16>, Vec<u16>> =
                ImageBuffer::from_vec(6216 as u32, 4168 as u32, unsafe {
                    slice::from_raw_parts((*imgdata).image, 4168 * 6216)
                        .iter()
                        .copied()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<u16>>()
                })
                .unwrap();
            Ok(FornaxRawImage::Rgba16(img))
        } else {
            miette::bail!("raw image are all null.")
        }
    }
}

