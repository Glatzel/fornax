// region:Initialization and denitialization
impl crate::Libraw {
    fn libraw_init() -> *mut libraw_sys::libraw_data_t { unsafe { libraw_sys::libraw_init(0) } }

    fn close(&self) { unsafe { libraw_sys::libraw_close(self.imgdata) } }
}
impl Drop for crate::Libraw {
    fn drop(&mut self) { self.close(); }
}
impl Default for crate::Libraw {
    fn default() -> Self { Self::new(None) }
}
