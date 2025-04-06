mod params;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{ffi::CString, path::Path};

use libraw::IDCRaw;
use libraw::ILibrawErrors;
use miette::{Context, IntoDiagnostic};
pub use params::DngConverterParams;
use path_slash::PathBufExt;
static DNG_CONVERTER_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
    let default_install_path =
        PathBuf::from("C:/Program Files/Adobe/Adobe DNG Converter/Adobe DNG Converter.exe");

    let exe = if default_install_path.exists() {
        default_install_path
    } else {
        // Try to get executable from environment variable.
        let e = std::env::var("DNG_CONVERTER")
            .into_diagnostic()
            .wrap_err("DNG converter is not installed.")
            .unwrap();
        let exe = PathBuf::from(e);
        if exe.exists() && exe.is_file() {
            exe
        } else {
            panic!("DNG converter is not installed.");
        }
    };
    clerk::debug!("Find dng converter: {}", exe.to_slash_lossy());
    exe
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

    pub fn params(&self) -> &DngConverterParams {
        &self.params
    }

    pub fn dng_file(&self, raw_file: &Path) -> miette::Result<PathBuf> {
        let mut file = if let Some(dir) = &self.params.directory {
            dir.clone()
        } else {
            PathBuf::from(raw_file.parent().unwrap())
        };
        if let Some(filename) = &self.params.filename {
            file.push(filename);
        } else {
            file.push(format!(
                "{}.dng",
                raw_file.file_stem().unwrap().to_str().unwrap()
            ));
        };
        clerk::debug!("Dng file: {}", file.to_slash_lossy());
        Ok(file)
    }
    pub fn convert_file(&self, raw_file: &PathBuf) -> miette::Result<PathBuf> {
        let raw_file = dunce::canonicalize(raw_file).into_diagnostic()?;

        let dng_file: PathBuf = self.dng_file(&raw_file)?;
        if self.params.overwrite {
            match std::fs::remove_file(&dng_file) {
                Ok(_) => {
                    clerk::info!(
                        "Remove(overwrite) existing dng file: {}",
                        self.dng_file(&raw_file)?.to_slash_lossy().to_string()
                    )
                }
                Err(_) => (),
            };
        }
        if !dng_file.exists() {
            let program = DNG_CONVERTER_EXECUTABLE.as_os_str();
            let args = self.params.to_cmd(&raw_file);
            let output = std::process::Command::new(program)
                .args(&args)
                .output()
                .into_diagnostic()?;
            clerk::debug!("Command:\n{:?} {}", program, &args.join(" "));
            clerk::debug!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            clerk::debug!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            if !&dng_file.exists() {
                miette::bail!("Dng conversion failed.");
            }
            clerk::debug!(
                "Write dng to: {}",
                dunce::canonicalize(&dng_file)
                    .into_diagnostic()?
                    .to_slash_lossy()
                    .to_string()
            );
        } else {
            clerk::info!(
                "DNG file already exists: {}",
                dunce::canonicalize(&dng_file)
                    .into_diagnostic()?
                    .to_slash_lossy()
                    .to_string()
            )
        }
        Ok(dng_file)
    }

    fn open_dng_file(&mut self, fname: &Path) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }
    pub fn unpack(&mut self) -> miette::Result<()> {
        Self::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }
}

impl fornax_core::IDecoder<&PathBuf> for DngConverter {
    fn decode(&mut self, file: &PathBuf) -> miette::Result<()> {
        let dng_file = self.convert_file(file)?;
        self.open_dng_file(&dng_file)?;
        self.unpack()?;
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
impl ILibrawErrors for DngConverter {}
