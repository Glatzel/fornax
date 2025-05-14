use std::ffi::CString;
use std::path::Path;
use std::slice;

use fornax_core::{BayerChannel, BayerPattern, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{EncodableLayout, ImageBuffer, Rgb};

use crate::{c_char_to_string, check_raw_alloc, check_run};
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
    pub(crate) params: Option<DCRawParams>,
}


// region:Methods Loading Data from a File
// https://www.libraw.org/docs/API-CXX.html#dataload
impl Libraw {
    pub fn open_file(&self, fname: &Path) -> miette::Result<&Self> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        check_run!(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        });
        Ok(self)
    }
    fn _open_file_ex(&self) -> miette::Result<&Self> { unimplemented!() }
    fn _open_wfile(&self) -> miette::Result<&Self> { unimplemented!() }
    fn _openwfile_ex(&self) -> miette::Result<&Self> { unimplemented!() }

    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<&Self> {
        check_run!(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        });
        Ok(self)
    }
    #[allow(clippy::too_many_arguments)]
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
        check_run!(unsafe {
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
                u8::from(procflags.clone()),
                bayer_pattern,
                unused_bits,
                otherflags,
                black_level,
            )
        });
        Ok(self)
    }

    pub fn unpack(&self) -> miette::Result<&Self> {
        check_run!(unsafe { libraw_sys::libraw_unpack(self.imgdata) });
        Ok(self)
    }
    pub fn unpack_thumb(&self) -> miette::Result<&Self> {
        check_run!(unsafe { libraw_sys::libraw_unpack_thumb(self.imgdata) });
        Ok(self)
    }
    fn _unpack_thumb_ex(&self) -> miette::Result<&Self> { unimplemented!() }
}
// region:Parameters setters/getters
impl Libraw {
    pub fn get_raw_height(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_height(self.imgdata) })
    }
    pub fn get_raw_width(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_width(self.imgdata) })
    }
    pub fn get_iheight(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iheight(self.imgdata) })
    }
    pub fn get_iwidth(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iwidth(self.imgdata) })
    }
    pub fn get_cam_mul(&self, index: BayerChannel) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_cam_mul(self.imgdata, u8::from(index) as i32) })
    }
    pub fn get_pre_mul(&self, index: BayerChannel) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_pre_mul(self.imgdata, u8::from(index) as i32) })
    }
    pub fn get_rgb_cam(&self, index1: i32, index2: i32) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_rgb_cam(self.imgdata, index1, index2) })
    }
    pub fn get_iparams(&self) -> miette::Result<LibrawIParams> {
        check_raw_alloc!(self.imgdata);
        LibrawIParams::new(self.imgdata)
    }
    pub fn get_lensinfo(&self) { unimplemented!() }
    pub fn get_imgother(&self) -> miette::Result<LibrawImgOther> {
        check_raw_alloc!(self.imgdata);
        LibrawImgOther::new(self.imgdata)
    }
    pub fn get_color_maximum(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_color_maximum(self.imgdata) })
    }
    pub fn set_user_mul(&self, index: i32, val: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_user_mul(self.imgdata, index, val) };
        self
    }
    pub fn set_demosaic(&self, value: DCRawUserQual) -> &Self {
        unsafe { libraw_sys::libraw_set_demosaic(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_adjust_maximum_thr(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_adjust_maximum_thr(self.imgdata, value) };
        self
    }
    pub fn set_output_color(&self, value: DCRawOutputColor) -> &Self {
        unsafe { libraw_sys::libraw_set_output_color(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_output_bps(&self, value: DCRawOutputBps) -> &Self {
        unsafe { libraw_sys::libraw_set_output_bps(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_gamma(&self, index: i32, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_gamma(self.imgdata, index, value) };
        self
    }
    pub fn set_no_auto_bright(&self, value: bool) -> &Self {
        unsafe { libraw_sys::libraw_set_no_auto_bright(self.imgdata, value as i32) };
        self
    }
    pub fn set_bright(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_bright(self.imgdata, value) };
        self
    }
    pub fn set_highlight(&self, value: DCRawHighlightMode) -> &Self {
        unsafe { libraw_sys::libraw_set_highlight(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_fbdd_noiserd(&self, value: DCRawFbddNoiserd) -> &Self {
        unsafe { libraw_sys::libraw_set_fbdd_noiserd(self.imgdata, i32::from(value)) };
        self
    }
}
// region:Auxiliary Functions
// https://www.libraw.org/docs/API-CXX.html#utility
impl Libraw {
    pub fn version() -> String { c_char_to_string(unsafe { libraw_sys::libraw_version() }) }
    fn _check_version() -> bool { unimplemented!() }
    fn _libraw_capabilities() { unimplemented!() }
    pub fn camera_count() -> i32 { unsafe { libraw_sys::libraw_cameraCount() } }
    pub fn camera_list() -> Vec<String> {
        let mut vec = Vec::new();
        let ptr = unsafe { libraw_sys::libraw_cameraList() };
        unsafe {
            let mut current_ptr = ptr;
            while !(*current_ptr).is_null() {
                vec.push(c_char_to_string(*current_ptr));
                current_ptr = current_ptr.add(1);
            }
        }
        vec
    }
    fn _libraw_get_decoder_info() { unimplemented!() }
    fn _libraw_unpack_function_name() { unimplemented!() }
    pub fn color(&self, row: i32, col: i32) -> i32 {
        unsafe { libraw_sys::libraw_COLOR(self.imgdata, row, col) }
    }
    pub fn libraw_subtract_black(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        unsafe { libraw_sys::libraw_subtract_black(self.imgdata) };
        Ok(self)
    }
    fn _libraw_recycle_datastream() { unimplemented!() }
    fn _libraw_recycle() { unimplemented!() }
    pub fn strerror(errorcode: i32) -> String {
        c_char_to_string(unsafe { libraw_sys::libraw_strerror(errorcode) })
    }
    fn _libraw_strprogress() { unimplemented!() }
    fn _libraw_set_dataerror_handler() { unimplemented!() }
    fn _libraw_set_progress_handler() { unimplemented!() }
}
// region:Data Postprocessing: Emulation of dcraw Behavior
//https://www.libraw.org/docs/API-CXX.html#dcrawemu
impl Libraw {
    pub fn raw2image(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata) });
        Ok(self)
    }
    fn _libraw_free_image() { unimplemented!() }
    fn _libraw_adjust_sizes_info_only() { unimplemented!() }
    pub fn dcraw_process(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        check_run!(unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) });
        Ok(self)
    }
}
//region:Writing to Output Files
impl Libraw {
    fn _libraw_dcraw_ppm_tiff_writer() { unimplemented!() }
    fn _libraw_dcraw_thumb_writer() { unimplemented!() }
}
//region:Writing processing results to memory buffer
impl Libraw {
    fn dcraw_make_mem_image(&self) -> miette::Result<DCRawProcessedImage> {
        check_raw_alloc!(self.imgdata);
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        check_run!(result);

        let processed = DCRawProcessedImage::new(processed)?;
        Ok(processed)
    }
    fn _libraw_dcraw_make_mem_thumb() { unimplemented!() }
    fn _libraw_dcraw_clear_mem() { unimplemented!() }
}
// region:Data Structure
impl Libraw {
    pub fn get_image_sizes(&self) -> miette::Result<LibrawImageSizes> {
        check_raw_alloc!(self.imgdata);
        LibrawImageSizes::new(self.imgdata)
    }

    pub fn get_rawdata(&self) -> miette::Result<LibrawRawdata> {
        check_raw_alloc!(self.imgdata);
        let size = self.get_image_sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        rawdata::LibrawRawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
}
// region:Custom API
impl Libraw {
    pub fn new(params: Option<DCRawParams>) -> Self {
        Self {
            imgdata: Self::libraw_init(),
            params,
        }
    }
    pub fn bayer_pattern(&self) -> miette::Result<fornax_core::BayerPattern> {
        check_raw_alloc!(self.imgdata);
        let pattern0 = self.color(0, 0);
        let pattern1 = self.color(0, 1);
        let pattern2 = self.color(1, 0);
        let pattern3 = self.color(1, 1);
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
        let raw_img = self.get_raw_image(true)?;
        let img = ImageBuffer::from_par_fn(raw_img.width(), raw_img.height(), |x, y| {
            let pixel = raw_img.get_pixel(x, y);
            let value = T::from(pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3])).unwrap();

            let value =
            // u16 -> u8
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
                value / T::from(255).unwrap()
            }
             // u16 -> u16
            else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>() {
                value
            }
            // u16 -> f32/f64
            else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>()
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
                            .map(|v| {
                                // u8 -> u8
                                if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u8>() {
                                    O::from(v).unwrap()
                                }
                                // u8 -> u16
                                else if std::any::TypeId::of::<O>()
                                    == std::any::TypeId::of::<u16>()
                                {
                                    O::from(v).unwrap() * O::from(255).unwrap()
                                }
                                // u8 -> f32/f64
                                else if std::any::TypeId::of::<O>()
                                    == std::any::TypeId::of::<f32>()
                                    || std::any::TypeId::of::<O>() == std::any::TypeId::of::<f64>()
                                {
                                    O::from(v).unwrap() / O::from(255).unwrap()
                                } else {
                                    panic!()
                                }
                            })
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
                    .map(|v| {
                        // u16 -> u8
                        if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u8>() {
                            O::from(v).unwrap() / O::from(255).unwrap()
                        }
                        // u16 -> u16
                        else if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u16>() {
                            O::from(v).unwrap()
                        }
                        // u16 -> f32/f64
                        else if std::any::TypeId::of::<O>() == std::any::TypeId::of::<f32>()
                            || std::any::TypeId::of::<O>() == std::any::TypeId::of::<f64>()
                        {
                            O::from(v).unwrap() / O::from(65536).unwrap()
                        } else {
                            panic!()
                        }
                    })
                    .collect(),
                )
                .unwrap();
                Ok(img)
            }
            (c, b) => miette::bail!("Unsupported color:{}, bits: {}.", c, b),
        }
    }
    pub fn get_raw_image(
        &self,
        subtract_black: bool,
    ) -> miette::Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>> {
        self.raw2image()?;
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata) });
        if subtract_black {
            self.libraw_subtract_black()?;
        }

        let size = self.get_image_sizes()?;
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
    pub fn get_params(&self) -> miette::Result<libraw_sys::libraw_output_params_t> {
        Ok(unsafe { (*self.imgdata).params })
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
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> { self.get_bayer_image() }
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
    fn bayer_image(&self) -> miette::Result<fornax_core::BayerImage<T>> { self.get_bayer_image() }
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

// region:Test
#[cfg(test)]
mod tests {
    use std::io::Read;

    use float_cmp::assert_approx_eq;
    use miette::IntoDiagnostic;

    use super::*;
    // region:Methods Loading Data from a File
    #[test]
    fn test_open_file() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?;
        Ok(())
    }
    #[test]
    pub fn test_open_buffer() -> miette::Result<()> {
        let mut file = std::fs::File::open(fornax_devtool::raw_file()).into_diagnostic()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).into_diagnostic()?;
        let libraw = Libraw::default();
        libraw.open_buffer(&buffer)?;
        Ok(())
    }
    #[test]
    fn test_unpack_thumb() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack_thumb()?;
        Ok(())
    }
    // region:Parameters setters/getters
    #[test]
    fn test_get_raw_height() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_height()?;
        assert_eq!(3516, value);
        Ok(())
    }
    #[test]
    fn test_get_raw_width() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_width()?;
        assert_eq!(5360, value);
        Ok(())
    }
    #[test]
    fn test_get_iheight() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iheight()?;
        assert_eq!(3464, value);
        Ok(())
    }
    #[test]
    fn test_get_iwidth() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iwidth()?;
        assert_eq!(5202, value);
        Ok(())
    }
    #[test]
    fn test_get_cam_mul() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;

        let value = libraw.get_cam_mul(BayerChannel::R)?;
        assert_eq!(2127.0, value);
        let value = libraw.get_cam_mul(BayerChannel::G)?;
        assert_eq!(1024.0, value);
        let value = libraw.get_cam_mul(BayerChannel::B)?;
        assert_eq!(1584.0, value);
        let value = libraw.get_cam_mul(BayerChannel::G2)?;
        assert_eq!(1024.0, value);
        Ok(())
    }
    #[test]
    fn test_get_pre_mul() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;

        let value = libraw.get_pre_mul(BayerChannel::R)?;
        assert_approx_eq!(f32, 2.1848476, value);
        let value = libraw.get_pre_mul(BayerChannel::G)?;
        assert_approx_eq!(f32, 0.9358199, value);
        let value = libraw.get_pre_mul(BayerChannel::B)?;
        assert_approx_eq!(f32, 1.2567647, value);
        let value = libraw.get_pre_mul(BayerChannel::G2)?;
        assert_approx_eq!(f32, 0.0, value);
        Ok(())
    }
    #[test]
    fn test_get_rgb_cam() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_rgb_cam(1, 2)?;
        assert_approx_eq!(f32, -0.5123857, value);
        Ok(())
    }
    #[test]
    fn test_get_color_maximum() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_color_maximum()?;
        assert_eq!(13584, value);
        Ok(())
    }
    #[test]
    fn test_set_user_mul() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_user_mul(1, 2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.user_mul[1]);
        Ok(())
    }
    #[test]
    fn test_set_demosaic() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_demosaic(DCRawUserQual::Linear);
        assert_eq!(
            i32::from(DCRawUserQual::Linear),
            libraw.get_params()?.user_qual
        );
        Ok(())
    }
    #[test]
    fn test_set_adjust_maximum_thr() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_adjust_maximum_thr(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.adjust_maximum_thr);
        Ok(())
    }
    #[test]
    fn test_set_output_color() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_color(DCRawOutputColor::ACES);
        assert_eq!(
            i32::from(DCRawOutputColor::ACES),
            libraw.get_params()?.output_color
        );
        Ok(())
    }
    #[test]
    fn set_output_bps() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_bps(DCRawOutputBps::_16bit);
        assert_eq!(
            i32::from(DCRawOutputBps::_16bit),
            libraw.get_params()?.output_bps
        );
        Ok(())
    }
    #[test]
    fn test_set_gamma() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_gamma(1, 2.0);
        assert_approx_eq!(f64, 2.0, libraw.get_params()?.gamm[1]);
        Ok(())
    }
    #[test]
    fn test_set_no_auto_bright() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_no_auto_bright(true);
        assert!(libraw.get_params()?.no_auto_bright != 0);
        Ok(())
    }
    #[test]
    fn test_set_bright() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_bright(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.bright);
        Ok(())
    }
    #[test]
    fn test_set_highlight() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_highlight(DCRawHighlightMode::Reconstruct4);
        assert_eq!(
            i32::from(DCRawHighlightMode::Reconstruct4),
            libraw.get_params()?.highlight
        );
        Ok(())
    }
    #[test]
    fn set_fbdd_noiserd() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_fbdd_noiserd(DCRawFbddNoiserd::Off);
        assert_eq!(
            i32::from(DCRawFbddNoiserd::Off),
            libraw.get_params()?.fbdd_noiserd
        );
        Ok(())
    }

    // region:Auxiliary Functions
    #[test]
    fn test_version() {
        let version = Libraw::version();
        assert_eq!(version, "0.21.4-Release".to_string());
    }
    #[test]
    fn test_camera_count() {
        let count = Libraw::camera_count();
        println!("camera_count: {}", count);
        assert!(count > 0);
    }
    #[test]
    fn test_camera_list() {
        let camera_list = Libraw::camera_list();
        println!("{:?}", camera_list);
        assert!(!camera_list.is_empty());
    }
    #[test]
    fn test_color() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw.open_file(&fornax_devtool::raw_file())?.color(0, 0);

        assert_eq!(value, 3);
        Ok(())
    }
}
