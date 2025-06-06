use std::path::Path;

use envoy::ToCString;
use fornax_core::BayerPattern;

use crate::{Libraw, check_run};

#[derive(Debug, Clone)]
pub enum ProcFlag {
    _10bit4PixelsIn5Bytes,
    _10bit6PixelsIn8Bytes,
    BigEndianData,
}
impl From<ProcFlag> for u8 {
    fn from(value: ProcFlag) -> Self {
        match value {
            ProcFlag::_10bit4PixelsIn5Bytes => 1,
            ProcFlag::_10bit6PixelsIn8Bytes => 0,
            ProcFlag::BigEndianData => 1,
        }
    }
}
// region:Methods Loading Data from a File
// https://www.libraw.org/docs/API-CXX.html#dataload
impl Libraw {
    pub fn open_file(&self, fname: &Path) -> miette::Result<&Self> {
        check_run!(unsafe {
            libraw_sys::libraw_open_file(
                self.imgdata,
                fname.to_str().unwrap().to_cstring().as_ptr(),
            )
        });
        Ok(self)
    }
    fn _open_file_ex(&self) -> miette::Result<&Self> { unimplemented!() }
    fn _open_wfile(&self) -> miette::Result<&Self> { unimplemented!() }
    fn _openwfile_ex(&self) -> miette::Result<&Self> { unimplemented!() }

    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<&Self> {
        check_run!(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        });
        Ok(self)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn open_bayer(
        &self,
        data: &[u8],
        raw_width: u16,
        raw_height: u16,
        left_margin: u16,
        top_margin: u16,
        right_margin: u16,
        bottom_margin: u16,
        procflags: ProcFlag,
        bayer_pattern: &BayerPattern,
        unused_bits: u32,
        otherflags: u32,
        black_level: u32,
    ) -> miette::Result<&Self> {
        let datalen = data.len();
        let data = data.as_ptr() as *mut std::ffi::c_uchar;
        let bayer_pattern = match bayer_pattern {
            BayerPattern::RGGB => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_RGGB as u8,
            BayerPattern::BGGR => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_BGGR as u8,
            BayerPattern::GRBG => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_GRBG as u8,
            BayerPattern::GBRG => libraw_sys::LibRaw_openbayer_patterns_LIBRAW_OPENBAYER_GBRG as u8,
        };
        check_run!(unsafe {
            libraw_sys::libraw_open_bayer(
                self.imgdata,
                data,
                datalen as std::ffi::c_uint,
                raw_width,
                raw_height,
                left_margin,
                top_margin,
                right_margin,
                bottom_margin,
                u8::from(procflags.clone()),
                bayer_pattern,
                unused_bits,
                otherflags,
                black_level,
            )
        });
        Ok(self)
    }

    pub fn unpack(&self) -> miette::Result<&Self> {
        check_run!(unsafe { libraw_sys::libraw_unpack(self.imgdata) });
        Ok(self)
    }
    pub fn unpack_thumb(&self) -> miette::Result<&Self> {
        check_run!(unsafe { libraw_sys::libraw_unpack_thumb(self.imgdata) });
        Ok(self)
    }
    fn _unpack_thumb_ex(&self) -> miette::Result<&Self> { unimplemented!() }
}
#[cfg(test)]
mod test {
    use std::io::Read;

    use miette::IntoDiagnostic;

    #[test]
    fn test_open_file() -> miette::Result<()> {
        let libraw = crate::Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?;
        Ok(())
    }
    #[test]
    pub fn test_open_buffer() -> miette::Result<()> {
        let mut file = std::fs::File::open(fornax_devtool::raw_file()).into_diagnostic()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).into_diagnostic()?;
        let libraw = crate::Libraw::default();
        libraw.open_buffer(&buffer)?;
        Ok(())
    }
    #[test]
    fn test_unpack_thumb() -> miette::Result<()> {
        let libraw = crate::Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack_thumb()?;
        Ok(())
    }
}
