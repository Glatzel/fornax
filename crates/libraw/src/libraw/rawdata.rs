use core::slice;

use fornax_core::FornaxRawData;
use image::ImageBuffer;

pub struct LibrawRawdata {}
impl LibrawRawdata {
    pub(crate) fn get_rawdata(
        imgdata: *mut libraw_sys::libraw_data_t,
        width: usize,
        height: usize,
    ) -> miette::Result<FornaxRawData> {
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
            Ok(img)
        } else {
            miette::bail!("Can not get rawdata, please try rawimage.")
        }
    }
}
