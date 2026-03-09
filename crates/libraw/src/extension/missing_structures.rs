use crate::{ImageSizes, Libraw, LibrawError, Rawdata, check_raw_alloc};

// region:Data Structure
impl Libraw {
    pub fn get_image_sizes(&self) -> Result<ImageSizes, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        ImageSizes::new(self.arc_imgdata_ptr())
    }

    pub fn get_rawdata(&self) -> Result<Rawdata, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        let size = self.get_image_sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        Rawdata::get_rawdata(self.arc_imgdata_ptr(), width as usize, height as usize)
    }
}
