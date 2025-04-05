use std::io::Read;
use std::path::PathBuf;

use libraw::IDCRaw;
use miette::IntoDiagnostic;
use sha2::{Digest, Sha256};

pub struct DngConverter {
    imgdata: Option<*mut libraw_sys::libraw_data_t>,
    raw_file: PathBuf,
    hash: String,
}
impl DngConverter {
    pub fn new(raw_file: PathBuf) -> miette::Result<Self> {
        let hash = DngConverter::calculate_hash(&raw_file)?;
        Ok(Self {
            imgdata: None,
            raw_file,
            hash,
        })
    }
    fn calculate_hash(raw_file: &PathBuf) -> miette::Result<String> {
        let mut file = std::fs::File::open(raw_file).into_diagnostic()?;
        let mut hasher = Sha256::new();

        // Buffer to read the file in chunks
        let mut buffer = [0u8; 4096];
        loop {
            let bytes_read = file.read(&mut buffer).into_diagnostic()?;
            if bytes_read == 0 {
                break; // End of file reached
            }
            hasher.update(&buffer[..bytes_read]);
        }

        // Finalize and get the hash result
        let result = hasher.finalize();

        // Convert the result to a hexadecimal string
        Ok(format!("{:x}", result))
    }
    pub fn raw_file(&self) -> &PathBuf {
        &self.raw_file
    }
    pub fn hash(&self) -> &str {
        &self.hash
    }
}
impl fornax_core::IDecoder for DngConverter {
    fn decode_file(&mut self, _file: std::path::PathBuf) -> miette::Result<()> {
        todo!()
    }

    fn decode_buffer(&mut self, _buf: &[u8]) -> miette::Result<()> {
        todo!()
    }
}
impl IDCRaw for DngConverter {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t> {
        if let Some(imgdata) = self.imgdata {
            Ok(imgdata)
        } else {
            miette::bail!("`imgdata` is null.")
        }
    }
}
