use envoy::{CStrListToVecString, CStrToString};

use crate::{Libraw, check_raw_alloc, check_run};

// region:Auxiliary Functions
// https://www.libraw.org/docs/API-CXX.html#utility
impl Libraw {
    pub fn version() -> String {
        unsafe { libraw_sys::libraw_version().to_string().unwrap_or_default() }
    }
    fn _check_version() -> bool { unimplemented!() }
    fn _libraw_capabilities() { unimplemented!() }
    pub fn camera_count() -> i32 { unsafe { libraw_sys::libraw_cameraCount() } }
    pub fn camera_list() -> Vec<String> {
        unsafe { libraw_sys::libraw_cameraList().cast_const().to_vec_string() }
    }
    fn _libraw_get_decoder_info() { unimplemented!() }
    fn _libraw_unpack_function_name() { unimplemented!() }
    pub fn color(&self, row: i32, col: i32) -> i32 {
        unsafe { libraw_sys::libraw_COLOR(self.imgdata, row, col) }
    }
    pub fn libraw_subtract_black(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        unsafe { libraw_sys::libraw_subtract_black(self.imgdata) };
        Ok(self)
    }
    fn _libraw_recycle_datastream() { unimplemented!() }
    fn _libraw_recycle() { unimplemented!() }
    pub fn strerror(errorcode: i32) -> String {
        unsafe {
            libraw_sys::libraw_strerror(errorcode)
                .to_string()
                .unwrap_or_default()
        }
    }
    fn _libraw_strprogress() { unimplemented!() }
    fn _libraw_set_dataerror_handler() { unimplemented!() }
    fn _libraw_set_progress_handler() { unimplemented!() }
}
// region:Data Postprocessing: Emulation of dcraw Behavior
//https://www.libraw.org/docs/API-CXX.html#dcrawemu
impl Libraw {
    pub fn raw2image(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata) });
        Ok(self)
    }
    fn _libraw_free_image() { unimplemented!() }
    fn _libraw_adjust_sizes_info_only() { unimplemented!() }
    pub fn dcraw_process(&self) -> miette::Result<&Self> {
        check_raw_alloc!(self.imgdata);
        check_run!(unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) });
        Ok(self)
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_version() {
        let version = Libraw::version();
        assert_eq!(version, "0.21.4-Release".to_string());
    }
    #[test]
    fn test_camera_count() {
        let count = Libraw::camera_count();
        println!("camera_count: {count}");
        assert!(count > 0);
    }
    #[test]
    fn test_camera_list() {
        let camera_list = Libraw::camera_list();
        println!("{camera_list:?}");
        assert!(!camera_list.is_empty());
    }
    #[test]
    fn test_color() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw.open_file(&fornax_devtool::raw_file())?.color(0, 0);

        assert_eq!(value, 3);
        Ok(())
    }
}
