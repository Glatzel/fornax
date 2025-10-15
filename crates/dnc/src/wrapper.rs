use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use path_slash::PathBufExt;

use crate::{DncError, DncParams};
static DNC_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path = std::env::var("PATH").unwrap_or_default();
    #[cfg(target_os = "macos")]
    path.insert_str(0, "/Applications/Adobe DNG Converter.app/Contents/MacOS:");
    #[cfg(target_os = "windows")]
    path.insert_str(0, r"C:\Program Files\Adobe\Adobe DNG Converter;");

    unsafe { std::env::set_var("PATH", &path) };
    PathBuf::from("Adobe DNG Converter")
});
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dnc {
    params: DncParams,
}
impl Dnc {
    pub fn new(params: DncParams) -> Self { Self { params } }

    pub fn params(&self) -> &DncParams { &self.params }

    fn dng_file(&self, raw_file: &Path) -> Result<PathBuf, DncError> {
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
    pub fn convert(&self, raw_file: &Path) -> Result<PathBuf, DncError> {
        let raw_file = dunce::canonicalize(raw_file)?;

        // Skip dng file
        if raw_file.extension().unwrap().eq_ignore_ascii_case("dng") {
            clerk::info!("The input file is dng.");
            return Ok(raw_file.clone());
        }

        let dng_file: PathBuf = self.dng_file(&raw_file)?;

        // Remove dng file if overwrite set to true
        if self.params.overwrite && std::fs::remove_file(&dng_file).is_ok() {
            clerk::info!(
                "Remove(overwrite) existing dng file: {}",
                self.dng_file(&raw_file)?.to_slash_lossy().to_string()
            );
        }

        // Execute dng converter to generate dng file.
        if !dng_file.exists() {
            let program = DNC_EXECUTABLE.as_os_str();
            let args = self.params.to_cmd(&raw_file)?;
            let _output = std::process::Command::new(program).args(&args).output()?;
            clerk::debug!("Command:\n{:?} {}", program, &args.join(" "));
            clerk::debug!("Stdout:\n{}", String::from_utf8_lossy(&_output.stdout));
            clerk::debug!("Stderr:\n{}", String::from_utf8_lossy(&_output.stderr));
            if !&dng_file.exists() {
                return Err(DncError::ConversionFailed);
            }
            clerk::debug!(
                "Write dng to: {}",
                dunce::canonicalize(&dng_file)?.to_slash_lossy().to_string()
            );
        } else {
            // Skip if dng file exists
            clerk::info!(
                "DNG file already exists: {}",
                dunce::canonicalize(&dng_file)?.to_slash_lossy().to_string()
            );
        }
        Ok(dng_file)
    }
}

impl Default for Dnc {
    fn default() -> Self { Self::new(DncParams::default()) }
}
