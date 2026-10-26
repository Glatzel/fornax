use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct ImgdataPtr(*mut libraw_sys::libraw_data_t);
impl ImgdataPtr {
    pub(crate) const fn ptr(&self) -> *mut libraw_sys::libraw_data_t { self.0 }
}
impl Deref for ImgdataPtr {
    type Target = libraw_sys::libraw_data_t;

    fn deref(&self) -> &Self::Target { unsafe { &*self.0 } }
}
impl Drop for ImgdataPtr {
    fn drop(&mut self) { unsafe { libraw_sys::libraw_close(self.0) } }
}
unsafe impl Send for ImgdataPtr {}
unsafe impl Sync for ImgdataPtr {}
/// # References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_data_t>
#[derive(Debug)]
pub struct Libraw {
    imgdata_ptr: Arc<ImgdataPtr>,
}
impl Libraw {
    pub(crate) fn arc_imgdata_ptr(&self) -> Arc<ImgdataPtr> { self.imgdata_ptr.clone() }
    pub(crate) fn imgdata_ptr(&self) -> *mut libraw_sys::libraw_data_t { self.imgdata_ptr.0 }
    fn libraw_init() -> Arc<ImgdataPtr> {
        let ptr = unsafe { libraw_sys::libraw_init(0) };
        assert!(!ptr.is_null(), "LibRaw initialization failed");
        Arc::new(ImgdataPtr(ptr))
    }
    pub fn new() -> Self {
        Self {
            imgdata_ptr: Self::libraw_init(),
        }
    }
}
impl Default for Libraw {
    fn default() -> Self { Self::new() }
}
