mod output_params;
mod processed_image;

use fornax_core::{FornaxProcessedImage, IDecoder, IPostProcessor};
pub use output_params::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawParams,
    DCRawUserFlip, DCRawUserQual,
};
pub use processed_image::{DCRawImageFormats, DCRawProcessedImage};

use crate::ILibrawErrors;

pub trait IDCRaw {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t>;
}
#[derive(Default)]
pub struct DCRaw {
    pub(crate) params: Option<DCRawParams>,
}
impl DCRaw {
    pub fn new(params: DCRawParams) -> Self {
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
    ) -> miette::Result<DCRawProcessedImage> {
        self.set_output_params_unsafe(imgdata)?;

        Self::check_run(
            unsafe { libraw_sys::libraw_dcraw_process(imgdata) },
            "libraw_dcraw_process",
        )?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(imgdata, &mut result) };
        Self::check_run(result, "libraw_dcraw_make_mem_image")?;

        let processed = DCRawProcessedImage::new(processed)?;
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
    ) -> miette::Result<DCRawProcessedImage> {
        self.dcraw_process_unsafe(imgdata)
    }
}
impl<D> IPostProcessor<D> for DCRaw
where
    D: crate::IDCRaw + IDecoder,
{
    fn post_process(&self, libraw: &D) -> miette::Result<FornaxProcessedImage> {
        let imgdata = libraw.imgdata()?;
        let processed = self.dcraw_process_unsafe(imgdata)?.to_image()?;
        Ok(processed)
    }
}
impl ILibrawErrors for DCRaw {}
