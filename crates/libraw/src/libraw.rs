mod image_sizes;
mod imgother;
mod iparams;
mod open_bayer_options;
mod rawdata;
use std::ffi::CString;
use std::path::Path;
use std::slice;
mod version;
use fornax_core::{BayerPattern, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{EncodableLayout, ImageBuffer, Rgb};
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::{ColorDesc, LibrawIParams};
pub use open_bayer_options::ProcFlag;
pub use rawdata::LibrawRawdata;
pub use version::LibrawVersion;

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
}
// region:Methods Loading Data from a File
// https://www.libraw.org/docs/API-CXX.html#dataload
impl Libraw {
    pub fn open_file(&self, fname: &Path) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(
            unsafe { libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _) },
            "libraw_open_file",
        )?;
        Ok(())
    }

    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(
            unsafe {
                libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
            },
            "libraw_open_buffer",
        )?;
        Ok(())
    }

    pub fn open_bayer(
        &self,
        data: &[u8],
        raw_width: u16,
        raw_height: u16,
        left_margin: u16,
        top_margin: u16,
        right_margin: u16,
        bottom_margin: u16,
        procflags: ProcFlag,
        bayer_pattern: &BayerPattern,
        unused_bits: u32,
        otherflags: u32,
        black_level: u32,
    ) -> miette::Result<()> {
        let datalen = data.len();
        let data = data.as_ptr() as *mut std::ffi::c_uchar;
        let bayer_pattern = match bayer_pattern {
            BayerPattern::RGGB => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_RGGB as u8,
            BayerPattern::BGGR => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_BGGR as u8,
            BayerPattern::GRBG => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_GRBG as u8,
            BayerPattern::GBRG => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_GBRG as u8,
        };
        Self::check_run(
            unsafe {
                libraw_sys::libraw_open_bayer(
                    self.imgdata,
                    data,
                    datalen as std::ffi::c_uint,
                    raw_width,
                    raw_height,
                    left_margin,
                    top_margin,
                    right_margin,
                    bottom_margin,
                    u8::from(procflags),
                    bayer_pattern,
                    unused_bits,
                    otherflags,
                    black_level,
                )
            },
            "libraw_open_buffer",
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
}
// region:Data Structure
impl Libraw {
    pub fn idata(&self) -> miette::Result<LibrawIParams> {
        LibrawIParams::new(self.imgdata)
    }
    pub fn other(&self) -> miette::Result<LibrawImgOther> {
        LibrawImgOther::new(self.imgdata)
    }
    pub fn sizes(&self) -> miette::Result<LibrawImageSizes> {
        LibrawImageSizes::new(self.imgdata)
    }

    pub fn rawdata(&self) -> miette::Result<Vec<LibrawRawdata>> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        let size = self.sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        rawdata::LibrawRawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
}
// region:Auxiliary Functions
// https://www.libraw.org/docs/API-CXX.html#utility
impl Libraw {
    pub fn version() -> LibrawVersion {
        LibrawVersion::new(
            libraw_sys::LIBRAW_MAJOR_VERSION,
            libraw_sys::LIBRAW_MINOR_VERSION,
            libraw_sys::LIBRAW_PATCH_VERSION,
        )
    }
}
// region:Data Postprocessing: Emulation of dcraw Behavior
//https://www.libraw.org/docs/API-CXX.html#dcrawemu
impl Libraw {
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
        let size = self.sizes()?;
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
}
// region:other
impl Libraw {
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
        T: FornaxPrimitive,
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

            let value = if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
                value / T::from(255).unwrap()
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>() {
                value
            } else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>()
                || std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>()
            {
                value / T::from(65535).unwrap()
            } else {
                panic!()
            };
            image::Luma::<T>([value])
        });
        Ok(fornax_core::BayerImage::new(img, pattern))
    }
    fn map_processed_image<O>(
        &self,
        processed: &DCRawProcessedImage,
    ) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>>
    where
        O: FornaxPrimitive,
    {
        match (processed.colors(), processed.bits()) {
            (3, 8) => {
                let img: image::ImageBuffer<image::Rgb<O>, Vec<O>> = image::ImageBuffer::from_vec(
                    processed.width() as u32,
                    processed.height() as u32,
                    unsafe {
                        slice::from_raw_parts(processed.data(), processed.data_size() as usize)
                            .to_vec()
                            .into_iter()
                            .map(|v| O::from(v).unwrap())
                            .collect()
                    },
                )
                .unwrap();
                Ok(img)
            }
            (3, 16) => {
                let img: image::ImageBuffer<image::Rgb<O>, Vec<O>> = image::ImageBuffer::from_vec(
                    processed.width() as u32,
                    processed.height() as u32,
                    bytemuck::cast_slice::<u8, u16>(unsafe {
                        slice::from_raw_parts(processed.data(), processed.data_size() as usize)
                    })
                    .to_vec()
                    .into_iter()
                    .map(|v| O::from(v).unwrap())
                    .collect(),
                )
                .unwrap();
                Ok(img)
            }
            (c, b) => miette::bail!("Unsupported color:{}, bits: {}.", c, b),
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
        Self::new(None)
    }
}
// region:fornax
impl<T> IDecoder<T> for Libraw
where
    T: FornaxPrimitive,
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
    T: FornaxPrimitive,
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

impl<D, O> IPostProcessor<D, u16, O> for Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>> {
        let bayer = decoder.bayer_image()?;
        self.open_bayer(
            bayer.mosaic().as_bytes(),
            bayer.mosaic().width() as u16,
            bayer.mosaic().height() as u16,
            0,
            0,
            0,
            0,
            ProcFlag::BigEndianData,
            bayer.pattern(),
            0,
            0,
            0,
        )?;
        self.unpack()?;
        let processed = self.dcraw_process()?;

        self.map_processed_image(&processed)
    }
}

impl<D, O> IPostProcessor<D, u16, O> for &Libraw
where
    D: IDecoder<u16>,
    O: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>> {
        let bayer = decoder.bayer_image()?;
        self.open_bayer(
            bayer.mosaic().as_bytes(),
            bayer.mosaic().width() as u16,
            bayer.mosaic().height() as u16,
            0,
            0,
            0,
            0,
            ProcFlag::BigEndianData,
            bayer.pattern(),
            0,
            0,
            0,
        )?;
        self.unpack()?;
        let processed = self.dcraw_process()?;
        self.map_processed_image(&processed)
    }
}
impl ILibrawErrors for Libraw {}
