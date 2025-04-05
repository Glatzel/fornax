use libraw::ILibraw;

pub struct DngConverter {
    imgdata: Option<*mut libraw_sys::libraw_data_t>,
}
impl fornax_core::IDecoder for DngConverter {
    fn decode_file(&mut self, file: std::path::PathBuf) -> miette::Result<()> {
        todo!()
    }

    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        todo!()
    }
}
impl ILibraw for DngConverter {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t> {
        if let Some(imgdata) = self.imgdata {
            Ok(imgdata)
        } else {
            miette::bail!("`imgdata` is null.")
        }
    }
}
