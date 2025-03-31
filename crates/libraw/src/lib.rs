use std::mem;

use libraw_sys as sys;

pub struct Processor {
    pub(crate) inner: *mut sys::libraw_data_t,
}
