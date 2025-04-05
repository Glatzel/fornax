mod output_params;
mod processed_image;
mod utils;


use fornax_traits::{IPostProcessor, ProcessedImage};
pub use output_params::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawParams,
    DCRawUserFlip, DCRawUserQual,
};
pub use processed_image::{DcRawProcessedImage, ImageFormats};

pub struct DcRaw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
}

impl IPostProcessor<Libraw, ProcessedImage> for DcRaw {
    fn post_process(&self, libraw: &Libraw) -> miette::Result<ProcessedImage> {
        libraw.dcraw_process()?.to_image()
    }
}
