use std::ops::Deref;
use std::slice;

use crate::{Fornax, ImageSizes};

pub struct RawImage {
    processor: Fornax,
}

impl RawImage {
    pub(crate) fn new(processor: Fornax) -> Self {
        debug_assert!(!unsafe { (*processor.imgdata).rawdata.raw_alloc }.is_null());

        Self { processor }
    }

    pub fn sizes(&self) -> ImageSizes {
        ImageSizes::new(unsafe { &(*self.processor.imgdata).sizes })
    }
}

impl Deref for RawImage {
    type Target = [u16];

    fn deref(&self) -> &Self::Target {
        let sizes = self.sizes();

        unsafe {
            slice::from_raw_parts(
                (*self.processor.imgdata).rawdata.raw_image,
                sizes.raw_width as usize * sizes.raw_height as usize,
            )
        }
    }
}
