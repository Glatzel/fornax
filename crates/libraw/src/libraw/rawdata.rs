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
        if !unsafe { (*imgdata).rawdata.raw_image }.is_null() {
            clerk::debug!("Found mono16 raw image.");
            let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> = {
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.raw_image, width * height).to_vec()
                })
                .unwrap()
            };
            Ok(FornaxRawImage::Mono16(img))
        } else if !unsafe { (*imgdata).rawdata.float_image }.is_null() {
            clerk::debug!("Found mono32 raw image.");
            let img: image::ImageBuffer<image::Luma<f32>, Vec<f32>> = {
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.float_image, width * height).to_vec()
                })
                .unwrap()
            };
            Ok(FornaxRawImage::MonoF32(img))
        } else if !unsafe { (*imgdata).rawdata.color3_image }.is_null() {
            clerk::debug!("Found rgb16 raw image.");
            let img: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.color3_image, width * height)
                        .iter()
                        .copied()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<u16>>()
                })
                .unwrap();
            Ok(FornaxRawImage::Rgb16(img))
        } else if unsafe { (*imgdata).rawdata.color4_image }.is_null() {
            clerk::debug!("Found rgba16 raw image.");
            let img: image::ImageBuffer<image::Rgba<u16>, Vec<u16>> =
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.color4_image, width * height)
                        .iter()
                        .copied()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<u16>>()
                })
                .unwrap();
            Ok(FornaxRawImage::Rgba16(img))
        } else if !unsafe { (*imgdata).rawdata.float3_image }.is_null() {
            clerk::debug!("Found rgb32 raw image.");
            let img: image::ImageBuffer<image::Rgb<f32>, Vec<f32>> =
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.float3_image, width * height)
                        .iter()
                        .copied()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<f32>>()
                })
                .unwrap();
            Ok(FornaxRawImage::RgbF32(img))
        } else if !unsafe { (*imgdata).rawdata.float4_image }.is_null() {
            clerk::debug!("Found rgba32 raw image.");
            let img: image::ImageBuffer<image::Rgba<f32>, Vec<f32>> =
                ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                    slice::from_raw_parts((*imgdata).rawdata.float4_image, width * height)
                        .iter()
                        .copied()
                        .flat_map(|pixel| pixel.into_iter())
                        .collect::<Vec<f32>>()
                })
                .unwrap();
            Ok(FornaxRawImage::RgbaF32(img))
        } else {
            miette::bail!("raw image are all null.")
        }
    }
}
