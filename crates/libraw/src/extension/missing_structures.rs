use crate::extension::error_handling::check_raw_alloc;
use crate::{ImageSizes, Libraw, LibrawError};

// region:Data Structure
impl Libraw {
    pub fn get_image_sizes(&self) -> Result<ImageSizes, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        ImageSizes::new(self.arc_imgdata_ptr())
    }
}
