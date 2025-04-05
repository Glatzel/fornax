pub struct DngConverter {}
impl fornax_core::IDecoder for DngConverter {
    fn decode_file(&mut self, file: std::path::PathBuf) -> miette::Result<()> {
        todo!()
    }

    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        todo!()
    }
}
