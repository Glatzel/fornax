use crate::{Libraw, LibrawImageSizes, LibrawRawdata, check_raw_alloc};

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
        LibrawRawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
}
