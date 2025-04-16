mod image_sizes;
mod imgother;
mod iparams;
mod rawdata;
use std::ffi::CString;
use std::path::Path;
use std::slice;

use fornax_core::{BayerImage, BayerPrimitive, IDecoder, IPostProcessor, ProcessedImage};
use image::ImageBuffer;
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::{ColorDesc, LibrawIParams};
pub use rawdata::LibrawRawdata;

use crate::ILibrawErrors;
use crate::dcraw::{DCRawParams, DCRawProcessedImage};
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
    pub(crate) params: Option<DCRawParams>,
}

impl Libraw {
    pub fn new(params: Option<DCRawParams>) -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata, params }
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
    // io
    pub fn open_bayer(
        &self,
        _bayer: BayerImage<u16>,
        // _raw_width: u16,
        // _raw_height: u16,
        // _left_margin: u16,
        // _top_margin: u16,
        // _right_margin: u16,
        // _bottom_margin: u16,
        // _procflags: u8,
        // _bayer_battern: u8,
        // _unused_bits: u32,
        // _otherflags: u32,
        // _black_level: u32,
    ) -> miette::Result<()> {
        todo!();
        // Self::check_run(
        //     unsafe {
        //         libraw_sys::libraw_open_bayer(self.imgdata,)
        //     },
        //     "libraw_open_buffer",
        // )?;
        // Ok(())
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
    pub fn rawdata(&self) -> miette::Result<Vec<LibrawRawdata>> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        let size = self.image_sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        rawdata::LibrawRawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
    pub fn raw2image(
        &self,
        subtract_black: bool,
    ) -> miette::Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("rawdata is null.")
        }
        Self::check_run(
            unsafe { libraw_sys::libraw_raw2image(self.imgdata) },
            "libraw_raw2image",
        )?;
        if subtract_black {
            unsafe { libraw_sys::libraw_subtract_black(self.imgdata) };
        }

        if unsafe { (*self.imgdata).image }.is_null() {
            miette::bail!("raw image is null.")
        }
        let size = self.image_sizes()?;
        let width = size.iwidth();
        let height = size.iheight();
        clerk::debug!("Width: {width}, Height: {height}");

        clerk::debug!("Found rgba16 raw image.");
        let img: ImageBuffer<image::Rgba<u16>, Vec<u16>> =
            ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                slice::from_raw_parts(
                    (*self.imgdata).image as *const u16,
                    width as usize * height as usize * 4,
                )
                .to_vec()
            })
            .unwrap();
        Ok(img)
    }
    fn dcraw_process(&self) -> miette::Result<DCRawProcessedImage> {
        if let Some(params) = &self.params {
            params.set_output_params(self.imgdata)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*self.imgdata).params });

        Self::check_run(
            unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) },
            "libraw_dcraw_process",
        )?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        Self::check_run(result, "libraw_dcraw_make_mem_image")?;

        let processed = DCRawProcessedImage::new(processed)?;
        Ok(processed)
    }
    pub fn bayer_pattern(&self) -> miette::Result<fornax_core::BayerPattern> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("rawdata is null.")
        }
        let pattern0 = Self::check_run(
            unsafe { libraw_sys::libraw_COLOR(self.imgdata, 0, 0) },
            "libraw_COLOR",
        )?;
        let pattern1 = Self::check_run(
            unsafe { libraw_sys::libraw_COLOR(self.imgdata, 0, 1) },
            "libraw_COLOR",
        )?;
        let pattern2 = Self::check_run(
            unsafe { libraw_sys::libraw_COLOR(self.imgdata, 1, 0) },
            "libraw_COLOR",
        )?;
        let pattern3 = Self::check_run(
            unsafe { libraw_sys::libraw_COLOR(self.imgdata, 1, 1) },
            "libraw_COLOR",
        )?;
        match (pattern0, pattern1, pattern2, pattern3) {
            (0, 1, 3, 2) => Ok(fornax_core::BayerPattern::RGGB),
            (2, 3, 1, 0) => Ok(fornax_core::BayerPattern::BGGR),
            (1, 0, 2, 3) => Ok(fornax_core::BayerPattern::GRBG),
            (3, 2, 0, 1) => Ok(fornax_core::BayerPattern::GBRG),
            (a, b, c, d) => miette::bail!("Unknown bayer pattern: {a}, {b}, {c}, {d}"),
        }
    }
    pub fn get_bayer_image<T>(&self) -> miette::Result<fornax_core::BayerImage<T>>
    where
        T: BayerPrimitive,
    {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        unsafe { libraw_sys::libraw_subtract_black(self.imgdata) };

        let pattern = self.bayer_pattern()?;
        let raw_img = self.raw2image(true)?;
        let img = ImageBuffer::from_par_fn(raw_img.width(), raw_img.height(), |x, y| {
            let pixel = raw_img.get_pixel(x, y);
            let value = T::from(pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3])).unwrap();
            image::Luma::<T>([value])
        });
        Ok(fornax_core::BayerImage::new(img, pattern))
    }
}

impl Drop for Libraw {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}
impl Default for Libraw {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T> IDecoder<T> for Libraw
where
    T: BayerPrimitive,
{
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
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> {
        self.get_bayer_image()
    }
}
impl<T> IDecoder<T> for &Libraw
where
    T: BayerPrimitive,
{
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
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> {
        self.get_bayer_image()
    }
}
impl IPostProcessor<Libraw, u8> for Libraw {
    fn post_process(&self, decoder: &Libraw) -> miette::Result<ProcessedImage> {
        let processed = decoder.dcraw_process()?.to_image()?;
        Ok(processed)
    }
}
impl IPostProcessor<&Libraw, u8> for &Libraw {
    fn post_process(&self, decoder: &&Libraw) -> miette::Result<ProcessedImage> {
        let processed = decoder.dcraw_process()?.to_image()?;
        Ok(processed)
    }
}
impl<D> IPostProcessor<D, u16> for Libraw
where
    D: IDecoder<u16>,
{
    fn post_process(&self, decoder: &D) -> miette::Result<ProcessedImage> {
        let bayer = decoder.bayer_image()?;
        self.open_bayer(bayer)?;
        self.unpack()?;
        let processed = self.dcraw_process()?.to_image()?;
        Ok(processed)
    }
}
impl ILibrawErrors for Libraw {}
