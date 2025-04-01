// use std::ffi::CString;
// use std::path::PathBuf;

// use libraw_sys as sys;

// use crate::error::LibrawError;
// use crate::raw_image::RawImage;
// pub type Result<T> = std::result::Result<T, LibrawError>;
// pub struct Fornax {
//     pub(crate) imgdata: *mut sys::libraw_data_t,
// }
// impl Fornax {
//     pub fn new() -> Self {
//         let imgdata = unsafe { sys::libraw_init(0) };
//         Self { imgdata }
//     }

//     pub fn open_buffer(&self, buf: &[u8]) -> Result<()> {
//         LibrawError::check(unsafe {
//             sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
//         })?;
//         Ok(())
//     }

//     pub fn open_file(&self, fname: PathBuf) -> Result<()> {
//         let c_string =
//             CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
//         LibrawError::check(unsafe {
//             sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
//         })?;
//         Ok(())
//     }

//     pub fn unpack(&self) -> Result<()> {
//         LibrawError::check(unsafe { sys::libraw_unpack(self.imgdata) })?;
//         Ok(())
//     }
//     pub fn raw_image(self) -> Result<RawImage> {
//         let decoded = RawImage::new(self);
//         Ok(decoded)
//     }
// }
