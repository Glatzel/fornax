use std::sync::Arc;

use crate::{ImgdataPointer, Libraw};

// region:Initialization and denitialization
impl Libraw {
    pub(crate) fn libraw_init() -> Arc<ImgdataPointer> {
        Arc::new(ImgdataPointer(unsafe { libraw_sys::libraw_init(0) }))
    }

    fn _close(&self) { unimplemented!() }
}
impl Drop for ImgdataPointer {
    fn drop(&mut self) { unsafe { libraw_sys::libraw_close(self.0) } }
}
impl Default for Libraw {
    fn default() -> Self { Self::new(None) }
}
