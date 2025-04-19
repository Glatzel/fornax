use std::ffi::{CStr, c_char};

pub(crate) fn c_char_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return "".to_string();
    }
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string()
}
