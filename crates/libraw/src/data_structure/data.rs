use std::sync::Arc;

use super::DCRawParams;
use crate::{LibrawError, LibrawErrorCode};

#[derive(Debug)]
pub struct ImgdataPtr(*mut libraw_sys::libraw_data_t);
impl ImgdataPtr {
    pub(crate) const fn ptr(&self) -> *mut libraw_sys::libraw_data_t { self.0 }
}
impl Drop for ImgdataPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { libraw_sys::libraw_close(self.0) }
        }
    }
}
unsafe impl Send for ImgdataPtr {}
unsafe impl Sync for ImgdataPtr {}
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
        Self::try_new(params).expect("LibRaw initialization failed")
    }

    /// Initializes a LibRaw context and reports allocation failure.
    ///
    /// # Errors
    ///
    /// Returns a LibRaw error when the native library cannot create a context.
    pub fn try_new(params: Option<DCRawParams>) -> Result<Self, LibrawError> {
        let imgdata_ptr = Self::libraw_init();
        if imgdata_ptr.0.is_null() {
            return Err(LibrawError::Libraw {
                code: LibrawErrorCode::UnsufficientMemory,
                message: "LibRaw initialization returned a null context".into(),
            });
        }
        Ok(Self {
            imgdata_ptr,
            params,
        })
    }
}
impl Default for Libraw {
    fn default() -> Self { Self::new(None) }
}
