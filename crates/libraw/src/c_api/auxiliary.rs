use envoy::{PtrListToVecString, PtrToString};

use crate::{Libraw, LibrawError, check_raw_alloc, check_run};

// region:Auxiliary Functions
// https://www.libraw.org/docs/API-CXX.html#utility
impl Libraw {
    pub fn version() -> Result<String, LibrawError> {
        unsafe { Ok(libraw_sys::libraw_version().to_string()?) }
    }
    fn _check_version() -> bool { todo!() }
    fn _libraw_capabilities() { todo!() }
    pub fn camera_count() -> i32 { unsafe { libraw_sys::libraw_cameraCount() } }
    pub fn camera_list() -> Result<Vec<String>, LibrawError> {
        unsafe {
            Ok(libraw_sys::libraw_cameraList()
                .cast_const()
                .to_vec_string()?)
        }
    }
    fn _libraw_get_decoder_info() { todo!() }
    fn _libraw_unpack_function_name() { todo!() }
    pub fn color(&self, row: i32, col: i32) -> i32 {
        unsafe { libraw_sys::libraw_COLOR(self.imgdata_ptr(), row, col) }
    }
    pub fn libraw_subtract_black(&self) -> Result<&Self, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        unsafe { libraw_sys::libraw_subtract_black(self.imgdata_ptr()) };
        Ok(self)
    }
    fn _libraw_recycle_datastream() { todo!() }
    fn _libraw_recycle() { todo!() }
    pub fn strerror(errorcode: i32) -> Result<String, LibrawError> {
        unsafe { Ok(libraw_sys::libraw_strerror(errorcode).to_string()?) }
    }
    fn _libraw_strprogress() { todo!() }
    fn _libraw_set_dataerror_handler() { todo!() }
    fn _libraw_set_progress_handler() { todo!() }
}
// region:Data Postprocessing: Emulation of dcraw Behavior
//https://www.libraw.org/docs/API-CXX.html#dcrawemu
impl Libraw {
    pub fn raw2image(&self) -> Result<&Self, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata_ptr()) });
        Ok(self)
    }
    fn _libraw_free_image() { todo!() }
    fn _libraw_adjust_sizes_info_only() { todo!() }
    pub fn dcraw_process(&self) -> Result<&Self, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        check_run!(unsafe { libraw_sys::libraw_dcraw_process(self.imgdata_ptr()) });
        Ok(self)
    }
}
#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_version() -> mischief::Result<()> {
        let version = Libraw::version()?;
        assert_eq!(
            version,
            format!(
                "{}.{}.{}-Release",
                libraw_sys::LIBRAW_MAJOR_VERSION,
                libraw_sys::LIBRAW_MINOR_VERSION,
                libraw_sys::LIBRAW_PATCH_VERSION,
            )
        );
        Ok(())
    }
    #[test]
    fn test_camera_count() {
        let count = Libraw::camera_count();
        println!("camera_count: {count}");
        assert!(count > 0);
    }
    #[test]
    fn test_camera_list() -> mischief::Result<()> {
        let camera_list = Libraw::camera_list()?;
        println!("{camera_list:?}");
        assert!(!camera_list.is_empty());
        Ok(())
    }
    #[test]
    fn test_color() -> mischief::Result<()> {
        let libraw = Libraw::default();
        let value = libraw.open_file(&fornax_devtool::raw_file())?.color(0, 0);

        assert_eq!(value, 3);
        Ok(())
    }
}
