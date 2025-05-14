use crate::{ImageSizes, Libraw, Rawdata, check_raw_alloc};

// region:Data Structure
impl Libraw {
    pub fn get_image_sizes(&self) -> miette::Result<ImageSizes> {
        check_raw_alloc!(self.imgdata);
        ImageSizes::new(self.imgdata)
    }

    pub fn get_rawdata(&self) -> miette::Result<Rawdata> {
        check_raw_alloc!(self.imgdata);
        let size = self.get_image_sizes()?;
        let width = size.raw_width();
        let height = size.raw_height();
        Rawdata::get_rawdata(self.imgdata, width as usize, height as usize)
    }
}
