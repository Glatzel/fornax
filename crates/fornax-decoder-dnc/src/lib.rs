mod params;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use fornax_core::metadata::Metadata;
use fornax_core::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder, Primitive};
use libraw::LibrawDecoder;
pub use params::DncParams;

static DNC_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
    let executable = "Adobe DNG Converter";

    #[cfg(target_os = "windows")]
    let executable = format!("{executable}.exe");

    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(&executable))
            .find(|path| path.is_file())
    }) {
        return path;
    }

    #[cfg(target_os = "macos")]
    {
        PathBuf::from(
            "/Applications/Adobe DNG Converter.app/Contents/MacOS/Adobe DNG Converter",
        )
    }

    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\Program Files\Adobe\Adobe DNG Converter.exe")
    }
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DncDecoder {
    params: DncParams,
}
impl DncDecoder {
    pub const fn new(params: DncParams) -> Self { Self { params } }

    pub const fn params(&self) -> &DncParams { &self.params }

    fn dng_file(&self, raw_file: &Path) -> PathBuf {
        let mut file = self.params.directory.as_ref().map_or_else(
            || PathBuf::from(raw_file.parent().unwrap()),
            core::clone::Clone::clone,
        );
        if let Some(filename) = &self.params.filename {
            file.push(filename);
        } else {
            file.push(format!(
                "{}.dng",
                raw_file.file_stem().unwrap().to_str().unwrap()
            ));
        }
        clerk::debug!("Dng file: {}", file.to_string_lossy());
        file
    }
    fn convert(&self, raw_file: &Path) -> Result<PathBuf, DecodeError> {
        let raw_file = std::path::absolute(raw_file)
            .map_err(|_| DecodeError::InvalidInput(raw_file.to_string_lossy().into()))?;

        // Skip dng file
        if raw_file.extension().unwrap().eq_ignore_ascii_case("dng") {
            clerk::info!("The input file is dng.");
            return Ok(raw_file);
        }

        let dng_file = self.dng_file(&raw_file);

        // Remove dng file if overwrite set to true
        if self.params.overwrite && std::fs::remove_file(&dng_file).is_ok() {
            clerk::info!(
                "Remove(overwrite) existing dng file: {}",
                self.dng_file(&raw_file).into_string().unwrap()
            );
        }

        // Execute dng converter to generate dng file.
        if dng_file.exists() {
            // Skip if dng file exists
            clerk::info!(
                "DNG file already exists: {}",
                std::path::absolute(&dng_file)
                    .map_err(|_| DecodeError::InvalidInput(raw_file.to_string_lossy().into()))?
                    .into_string()
                    .unwrap()
            );
        } else {
            let program = DNC_EXECUTABLE.as_os_str();
            let args = self.params.to_cmd(&raw_file);
            let _output = std::process::Command::new(program)
                .args(&args)
                .output()
                .unwrap();
            clerk::debug!("Command:\n{:?} {}", program, args.join(" "));
            clerk::debug!("Stdout:\n{}", String::from_utf8_lossy(&_output.stdout));
            clerk::debug!("Stderr:\n{}", String::from_utf8_lossy(&_output.stderr));
            if !&dng_file.exists() {
                return Err(DecodeError::Backend("Conversion failed.".into()));
            }
            clerk::debug!(
                "Write dng to: {}",
                std::path::absolute(&dng_file)
                    .unwrap()
                    .into_string()
                    .unwrap()
            );
        }
        Ok(dng_file)
    }
}

impl Default for DncDecoder {
    fn default() -> Self { Self::new(DncParams::default()) }
}
impl<T> IDecoder<Path, T> for DncDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        let dng_file = self.convert(input)?;
        LibrawDecoder.decode_full(&dng_file as &Path)
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        let dng_file = self.convert(input)?;
        LibrawDecoder.decode_active(&dng_file as &Path)
    }
}
