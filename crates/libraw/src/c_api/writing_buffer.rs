use crate::data_structure::{Libraw, LibrawError, ProcessedImage};
use crate::extension::error_handling::{check_raw_alloc, check_run};

//region:Writing processing results to memory buffer
impl Libraw {
    fn _dcraw_make_mem_image(&self) -> Result<ProcessedImage, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        let mut result = 0_i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata_ptr(), &raw mut result) };
        check_run!(result.into());

        let processed = ProcessedImage::new(processed)?;
        Ok(processed)
    }
    fn _libraw_dcraw_make_mem_thumb() { todo!() }
    fn _libraw_dcraw_clear_mem() { todo!() }
}
