use crate::Libraw;

// region:Initialization and denitialization
impl Libraw {
    pub(crate) fn libraw_init() -> *mut libraw_sys::libraw_data_t {
        unsafe { libraw_sys::libraw_init(0) }
    }

    fn close(&self) { unsafe { libraw_sys::libraw_close(self.imgdata) } }
}
impl Drop for Libraw {
    fn drop(&mut self) { self.close(); }
}
impl Default for Libraw {
    fn default() -> Self { Self::new(None) }
}
