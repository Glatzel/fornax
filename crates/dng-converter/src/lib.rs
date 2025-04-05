mod params;
use libraw::IDCRaw;
use miette::{Context, IntoDiagnostic};
pub use params::DngConverterParams;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::io::Read;
use std::path::PathBuf;
use std::sync::LazyLock;

static DNG_CONVERTER_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
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
    imgdata: *mut libraw_sys::libraw_data_t,
    params: DngConverterParams,
}
impl DngConverter {
    pub fn new(params: DngConverterParams) -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata, params }
    }
    fn calculate_file_hash(raw_file: &PathBuf) -> miette::Result<String> {
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
    fn calculate_buffer_hash(buf: &[u8]) -> miette::Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        // Finalize and get the hash result
        let result = hasher.finalize();
        // Convert the result to a hexadecimal string
        Ok(format!("{:x}", result))
    }
    pub fn params(&self) -> &DngConverterParams {
        &self.params
    }

    pub fn dng_file(&self, hash: &str) -> miette::Result<PathBuf> {
        let mut file = PathBuf::from(std::env::var("TEMP").into_diagnostic()?);
        file.push(format!("{}.dng", hash));
        Ok(file)
    }
    pub fn convert_file(&self, raw_file: &PathBuf) -> miette::Result<PathBuf> {
        let hash = Self::calculate_file_hash(raw_file)?;
        let dng_file = self.dng_file(&hash)?;
        if !dng_file.exists() {
            let program = DNG_CONVERTER_EXECUTABLE.as_os_str();
            let mut args = self.params.to_cmd();
            args.push("-d".to_string());
            args.push(dng_file.parent().unwrap().to_string_lossy().to_string());
            args.push("-o".to_string());
            args.push(format!("{}.dng", { hash }));
            args.push(raw_file.to_str().unwrap().to_string());
            let args = args.join(" ");
            let output = std::process::Command::new(program)
                .arg(&args)
                .output()
                .into_diagnostic()?;
            clerk::debug!("Command:\n{:?} {}", program, &args);
            clerk::debug!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            clerk::debug!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            clerk::debug!("Write dng to: {}", dng_file.to_str().unwrap())
        } else {
            clerk::info!("DNG file already exists: {}", dng_file.to_str().unwrap())
        }
        Ok(dng_file)
    }
    pub fn convert_buffer(&self, buf: &[u8]) -> miette::Result<PathBuf> {
        let hash = Self::calculate_buffer_hash(buf)?;
        let file = self.dng_file(&hash)?;
        let program = DNG_CONVERTER_EXECUTABLE.as_os_str();
        let mut args = self.params.to_cmd();
        args.push("-d".to_string());
        args.push(file.parent().unwrap().to_str().unwrap().to_string());
        args.push("-o".to_string());
        args.push(file.parent().unwrap().to_str().unwrap().to_string());
        let args = args.join(" ");
        if !file.exists() {
            let output = std::process::Command::new(program)
                .arg(args)
                .output()
                .into_diagnostic()?;
            clerk::debug!("Stdout:/n{}", String::from_utf8_lossy(&output.stdout));
            clerk::debug!("Stderr:/n{}", String::from_utf8_lossy(&output.stderr));
        } else {
            clerk::info!("DNG file already exists: {}", file.to_str().unwrap())
        }
        Ok(file)
    }
    fn open_dng_file(&mut self, fname: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        libraw::utils::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }
    // io
    pub fn open_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        libraw::utils::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }
}

impl fornax_core::IDecoder for DngConverter {
    fn decode_file(&mut self, file: std::path::PathBuf) -> miette::Result<()> {
        let dng_file = self.convert_file(&file)?;
        self.open_dng_file(dng_file)?;
        Ok(())
    }

    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        let dng_file = self.convert_buffer(buf)?;
        self.open_dng_file(dng_file)?;
        Ok(())
    }
}
impl IDCRaw for DngConverter {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t> {
        if !self.imgdata.is_null() {
            Ok(self.imgdata)
        } else {
            miette::bail!("`imgdata` is null.")
        }
    }
}
impl Default for DngConverter {
    fn default() -> Self {
        Self::new(DngConverterParams::default())
    }
}
impl Drop for DngConverter {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}
