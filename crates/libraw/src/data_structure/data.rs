use std::sync::Arc;

use super::DCRawParams;
#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct ImgdataPointer(pub *mut libraw_sys::libraw_data_t);

#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: Arc<ImgdataPointer>,
    pub(crate) params: Option<DCRawParams>,
}
