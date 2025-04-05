mod output_params;
mod processed_image;
use fornax_core::{IDecoder, IPostProcessor, ProcessedImage};
use libraw::{ILibraw, Libraw};
pub use output_params::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawParams,
    DCRawUserFlip, DCRawUserQual,
};
pub use processed_image::{DcRawProcessedImage, ImageFormats};
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
    fn set_output_params(&self, imgdata: *mut libraw_sys::libraw_data_t) -> miette::Result<()> {
        if let Some(params) = &self.params {
            params.set_output_params(imgdata)?;
        }
        clerk::debug!("Set new params.");
        clerk::debug!("{:?}", unsafe { (*imgdata).params });
        Ok(())
    }
    fn dcraw_process(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<DcRawProcessedImage> {
        self.set_output_params(imgdata)?;

        Libraw::check_run(unsafe { libraw_sys::libraw_dcraw_process(imgdata) })?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(imgdata, &mut result) };
        Libraw::check_run(result)?;

        let processed = DcRawProcessedImage::new(processed)?;
        Ok(processed)
    }
}
impl<T> IPostProcessor<T, ProcessedImage> for DCRaw
where
    T: ILibraw + IDecoder,
{
    fn post_process(&self, libraw: &T) -> miette::Result<ProcessedImage> {
        let imgdata = libraw.imgdata()?;
        let processed = self.dcraw_process(imgdata)?.to_image()?;
        Ok(processed)
    }
}
