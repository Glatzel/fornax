mod params;
use libraw::IDCRaw;
use miette::{Context, IntoDiagnostic};
pub use params::DngConverterParams;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use std::sync::LazyLock;

static _DNG_CONVERTER_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
    let default_install_path =
        PathBuf::from("C:/Program Files/Adobe/Adobe DNG Converter/Adobe DNG Converter.exe");

    if default_install_path.exists() {
        default_install_path
    } else {
        let e = std::env::var("DNG_CONVERTER")
            .into_diagnostic()
            .wrap_err("DNG converter is not installed.")
            .unwrap();
        PathBuf::from(e)
    }
});

pub struct DngConverter {
    imgdata: Option<*mut libraw_sys::libraw_data_t>,
    raw_file: PathBuf,
    hash: String,
    params: DngConverterParams,
}
impl DngConverter {
    pub fn new(raw_file: PathBuf, params: DngConverterParams) -> miette::Result<Self> {
        let hash = DngConverter::calculate_hash(&raw_file)?;
        Ok(Self {
            imgdata: None,
            raw_file,
            hash,
            params,
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
    pub fn params(&self) -> &DngConverterParams {
        &self.params
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
