use super::DCRawParams;
#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
    pub(crate) params: Option<DCRawParams>,
}
