use crate::{Libraw, LibrawError, ProcessedImage, check_raw_alloc, check_run};

//region:Writing processing results to memory buffer
impl Libraw {
    pub(crate) fn dcraw_make_mem_image(&self) -> Result<ProcessedImage, LibrawError> {
        check_raw_alloc!(self.imgdata);
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        check_run!(result);

        let processed = ProcessedImage::new(processed)?;
        Ok(processed)
    }
    fn _libraw_dcraw_make_mem_thumb() { unimplemented!() }
    fn _libraw_dcraw_clear_mem() { unimplemented!() }
}
