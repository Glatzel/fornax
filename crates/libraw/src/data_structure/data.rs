use std::sync::Arc;

use super::DCRawParams;
#[derive(Debug)]
pub(crate) struct ImgdataPtr(*mut libraw_sys::libraw_data_t);
impl ImgdataPtr {
    pub(crate) fn ptr(&self) -> *mut libraw_sys::libraw_data_t { self.0 }
}
/// # References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_data_t>
#[derive(Debug)]
pub struct Libraw {
    imgdata_ptr: Arc<ImgdataPtr>,
    pub(crate) params: Option<DCRawParams>,
}
impl Libraw {
    pub(crate) fn arc_imgdata_ptr(&self) -> Arc<ImgdataPtr> { self.imgdata_ptr.clone() }
    pub(crate) fn imgdata_ptr(&self) -> *mut libraw_sys::libraw_data_t { self.imgdata_ptr.0 }
    fn libraw_init() -> Arc<ImgdataPtr> {
        Arc::new(ImgdataPtr(unsafe { libraw_sys::libraw_init(0) }))
    }
    pub fn new(params: Option<DCRawParams>) -> Self {
        Self {
            imgdata_ptr: Self::libraw_init(),
            params,
        }
    }

    fn close(&self) { unsafe { libraw_sys::libraw_close(self.imgdata_ptr.0) } }
}
impl Drop for Libraw {
    fn drop(&mut self) { self.close(); }
}
impl Default for Libraw {
    fn default() -> Self { Self::new(None) }
}
