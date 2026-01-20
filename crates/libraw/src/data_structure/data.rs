use std::sync::Arc;

use super::DCRawParams;

pub(crate) type ImgdataPointer = *mut libraw_sys::libraw_data_t;
/// # References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_data_t>
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: Arc<ImgdataPointer>,
    pub(crate) params: Option<DCRawParams>,
}
