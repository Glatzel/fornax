mod image_sizes;
mod imgother;
mod iparams;
mod open_bayer_options;
mod rawdata;
use fornax_core::{BayerPattern, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{EncodableLayout, ImageBuffer, Rgb};
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::{ColorDesc, LibrawIParams};
pub use open_bayer_options::ProcFlag;
pub use rawdata::LibrawRawdata;
use std::ffi::CString;
use std::path::Path;
use std::slice;

use crate::ILibrawErrors;
use crate::dcraw::{DCRawParams, DCRawProcessedImage};
use crate::utils::c_char_to_string;
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
    pub(crate) params: Option<DCRawParams>,
}
// region:Initialization and denitialization
impl Libraw {
    fn libraw_init() -> *mut libraw_sys::libraw_data_t {
        unsafe { libraw_sys::libraw_init(0) }
    }

    fn close(&self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}

// region:Methods Loading Data from a File
// https://www.libraw.org/docs/API-CXX.html#dataload
impl Libraw {
    pub fn open_file(&self, fname: &Path) -> miette::Result<&Self> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(
            unsafe { libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _) },
            "libraw_open_file",
        )?;
        Ok(self)
    }
    pub fn _open_file_ex(&self) -> miette::Result<&Self> {
        unimplemented!()
    }
    pub fn open_wfile(&self) -> miette::Result<&Self> {
        unimplemented!()
    }
    fn _openwfile_ex(&self) -> miette::Result<&Self> {
        unimplemented!()
    }

    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<&Self> {
        Self::check_run(
            unsafe {
                libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
            },
            "libraw_open_buffer",
        )?;
        Ok(self)
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
    ) -> miette::Result<&Self> {
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
        Ok(self)
    }

    pub fn unpack(&self) -> miette::Result<&Self> {
        Self::check_run(
            unsafe { libraw_sys::libraw_unpack(self.imgdata) },
            "libraw_unpack",
        )?;
        Ok(self)
    }
    pub fn unpack_thumb(&self) -> miette::Result<&Self> {
        Self::check_run(
            unsafe { libraw_sys::libraw_unpack_thumb(self.imgdata) },
            "libraw_unpack_thumb",
        )?;
        Ok(self)
    }
    fn _unpack_thumb_ex(&self) -> miette::Result<&Self> {
        unimplemented!()
    }
}
// region:Parameters setters/getters
impl Libraw {
    pub fn get_raw_height(&self) -> miette::Result<i32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_raw_height(self.imgdata) })
    }
    pub fn get_raw_width(&self) -> miette::Result<i32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_raw_width(self.imgdata) })
    }
    pub fn get_iheight(&self) -> miette::Result<i32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_iheight(self.imgdata) })
    }
    pub fn get_iwidth(&self) -> miette::Result<i32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_iwidth(self.imgdata) })
    }
    pub fn get_cam_mul(&self, index: i32) -> miette::Result<f32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_cam_mul(self.imgdata, index) })
    }
    pub fn get_pre_mul(&self, index: i32) -> miette::Result<f32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_pre_mul(self.imgdata, index) })
    }
    pub fn get_rgb_cam(&self, index1: i32, index2: i32) -> miette::Result<f32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_rgb_cam(self.imgdata, index1, index2) })
    }
    pub fn get_iparams(&self) -> miette::Result<LibrawIParams> {
        Self::check_raw_alloc(self.imgdata)?;
        LibrawIParams::new(self.imgdata)
    }
    pub fn get_lensinfo(&self) {
        unimplemented!()
    }
    pub fn get_imgother(&self) -> miette::Result<LibrawImgOther> {
        Self::check_raw_alloc(self.imgdata)?;
        LibrawImgOther::new(self.imgdata)
    }
    pub fn get_color_maximum(&self) -> miette::Result<i32> {
        Self::check_raw_alloc(self.imgdata)?;
        Ok(unsafe { libraw_sys::libraw_get_color_maximum(self.imgdata) })
    }
    pub fn set_user_mul(&self, index: i32, val: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_user_mul(self.imgdata, index, val) };
        self
    }
    fn _set_demosaic() {
        unimplemented!()
    }
    fn _set_adjust_maximum_thr() {
        unimplemented!()
    }
    fn _set_output_color() {
        unimplemented!()
    }
    fn _set_output_bps() {
        unimplemented!()
    }
    fn _set_gamma() {
        unimplemented!()
    }
    fn _set_no_auto_bright() {
        unimplemented!()
    }
    fn _set_bright() {
        unimplemented!()
    }
    fn _set_highlight() {
        unimplemented!()
    }
    fn _set_fbdd_noiserd() {
        unimplemented!()
    }
}
// region:Auxiliary Functions
// https://www.libraw.org/docs/API-CXX.html#utility
impl Libraw {
    pub fn version() -> String {
        c_char_to_string(unsafe { libraw_sys::libraw_version() })
    }
    fn _check_version() -> bool {
        unimplemented!()
    }
    fn _libraw_capabilities() {
        unimplemented!()
    }
    fn _libraw_camera_count() {
        unimplemented!()
    }
    fn _libraw_camera_list() {
        unimplemented!()
    }
    fn _libraw_get_decoder_info() {
        unimplemented!()
    }
    fn _libraw_unpack_function_name() {
        unimplemented!()
    }
    pub fn libraw_color(&self, row: i32, col: i32) -> i32 {
        unsafe { libraw_sys::libraw_COLOR(self.imgdata, row, col) }
    }
    pub fn libraw_subtract_black(&self) -> miette::Result<&Self> {
        Self::check_raw_alloc(self.imgdata)?;
        unsafe { libraw_sys::libraw_subtract_black(self.imgdata) };
        Ok(self)
    }
    fn _libraw_recycle_datastream() {
        unimplemented!()
    }
    fn _libraw_recycle() {
        unimplemented!()
    }
    pub fn strerror(errorcode: i32) -> String {
        c_char_to_string(unsafe { libraw_sys::libraw_strerror(errorcode) })
    }
    fn _libraw_strprogress() {
        unimplemented!()
    }
    fn _libraw_set_dataerror_handler() {
        unimplemented!()
    }
    fn _libraw_set_progress_handler() {
        unimplemented!()
    }
}
// region:Data Postprocessing: Emulation of dcraw Behavior
//https://www.libraw.org/docs/API-CXX.html#dcrawemu
impl Libraw {
    pub fn raw2image(&self) -> miette::Result<&Self> {
        Self::check_raw_alloc(self.imgdata)?;
        Self::check_run(
            unsafe { libraw_sys::libraw_raw2image(self.imgdata) },
            "libraw_raw2image",
        )?;
        Ok(self)
    }
    fn _libraw_free_image() {
        unimplemented!()
    }
    fn _libraw_adjust_sizes_info_only() {
        unimplemented!()
    }
    pub fn dcraw_process(&self) -> miette::Result<&Self> {
        Self::check_raw_alloc(self.imgdata)?;
        Self::check_run(
            unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) },
            "libraw_dcraw_process",
        )?;
        Ok(self)
    }
}
//region:Writing to Output Files
impl Libraw {
    fn _libraw_dcraw_ppm_tiff_writer() {
        unimplemented!()
    }
    fn _libraw_dcraw_thumb_writer() {
        unimplemented!()
    }
}
//region:Writing processing results to memory buffer
impl Libraw {
    fn dcraw_make_mem_image(&self) -> miette::Result<DCRawProcessedImage> {
        Self::check_raw_alloc(self.imgdata)?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        Self::check_run(result, "libraw_dcraw_make_mem_image")?;

        let processed = DCRawProcessedImage::new(processed)?;
        Ok(processed)
    }
    fn _libraw_dcraw_make_mem_thumb() {
        unimplemented!()
    }
    fn _libraw_dcraw_clear_mem() {
        unimplemented!()
    }
}
// region:Data Structure
impl Libraw {
    pub fn sizes(&self) -> miette::Result<LibrawImageSizes> {
        Self::check_raw_alloc(self.imgdata)?;
        LibrawImageSizes::new(self.imgdata)
    }

    pub fn rawdata(&self) -> miette::Result<LibrawRawdata> {
        Self::check_raw_alloc(self.imgdata)?;
        let size = self.sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        rawdata::LibrawRawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
}
// region:Safe API
impl Libraw {
    pub fn new(params: Option<DCRawParams>) -> Self {
        Self {
            imgdata: Self::libraw_init(),
            params,
        }
    }
    pub fn bayer_pattern(&self) -> miette::Result<fornax_core::BayerPattern> {
        Self::check_raw_alloc(self.imgdata)?;
        let pattern0 = self.libraw_color(0, 0);
        let pattern1 = self.libraw_color(0, 1);
        let pattern2 = self.libraw_color(1, 0);
        let pattern3 = self.libraw_color(1, 1);
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
        let pattern = self.bayer_pattern()?;
        let raw_img = self.raw_image(true)?;
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
                            .iter()
                            .copied()
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
                    .iter()
                    .copied()
                    .map(|v| O::from(v).unwrap())
                    .collect(),
                )
                .unwrap();
                Ok(img)
            }
            (c, b) => miette::bail!("Unsupported color:{}, bits: {}.", c, b),
        }
    }
    pub fn raw_image(
        &self,
        subtract_black: bool,
    ) -> miette::Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>> {
        self.raw2image()?;
        Self::check_run(
            unsafe { libraw_sys::libraw_raw2image(self.imgdata) },
            "libraw_raw2image",
        )?;
        if subtract_black {
            self.raw2image()?;
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
}

impl Drop for Libraw {
    fn drop(&mut self) {
        self.close();
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
        if let Some(params) = &self.params {
            params.set_output_params(self.imgdata)?;
        }
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
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
        if let Some(params) = &self.params {
            params.set_output_params(self.imgdata)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*self.imgdata).params });
        let processed = self.dcraw_process()?.dcraw_make_mem_image()?;
        self.map_processed_image(&processed)
    }
}
impl ILibrawErrors for Libraw {}
