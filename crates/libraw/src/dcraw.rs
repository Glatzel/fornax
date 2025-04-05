mod output_params;
mod processed_image;
use fornax_core::{FornaxProcessedImage, IDecoder, IPostProcessor};
pub use output_params::{
    FbddNoiserd, HighlightMode, OutputBps, OutputColor, Params, UserFlip, UserQual,
};
pub use processed_image::{ImageFormats, ProcessedImage};

pub trait IDCRaw {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t>;
}
#[derive(Default)]
pub struct DCRaw {
    pub(crate) params: Option<Params>,
}
impl DCRaw {
    pub fn new(params: Params) -> Self {
        Self {
            params: Some(params),
        }
    }
}

impl DCRaw {
    fn set_output_params_unsafe(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<()> {
        if let Some(params) = &self.params {
            params.set_output_params(imgdata)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*imgdata).params });
        Ok(())
    }
    fn dcraw_process_unsafe(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<ProcessedImage> {
        self.set_output_params_unsafe(imgdata)?;

        crate::check_run(unsafe { libraw_sys::libraw_dcraw_process(imgdata) })?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(imgdata, &mut result) };
        crate::check_run(result)?;

        let processed = ProcessedImage::new(processed)?;
        Ok(processed)
    }
}
impl DCRaw {
    pub fn set_output_params(&self, imgdata: *mut libraw_sys::libraw_data_t) -> miette::Result<()> {
        self.set_output_params_unsafe(imgdata)
    }
    pub fn dcraw_process(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<ProcessedImage> {
        self.dcraw_process_unsafe(imgdata)
    }
}
impl<T> IPostProcessor<T, FornaxProcessedImage> for DCRaw
where
    T: crate::IDCRaw + IDecoder,
{
    fn post_process(&self, libraw: &T) -> miette::Result<FornaxProcessedImage> {
        let imgdata = libraw.imgdata()?;
        let processed = self.dcraw_process_unsafe(imgdata)?.to_image()?;
        Ok(processed)
    }
}
