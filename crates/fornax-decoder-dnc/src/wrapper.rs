use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use fornax_core::error::DecodeError;
use fornax_core::metadata::Metadata;
use fornax_core::{DecodedRawActive, DecodedRawFull, IDecoder, Primitive};
use fornax_decoder_rawler::RawlerDecoder;

pub use crate::DncParams;

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
        PathBuf::from("/Applications/Adobe DNG Converter.app/Contents/MacOS/Adobe DNG Converter")
    }

    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\Program Files\Adobe\Adobe DNG Converter\Adobe DNG Converter.exe")
    }
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DncDecoder {
    params: DncParams,
}
impl DncDecoder {
    pub const fn new(params: DncParams) -> Self { Self { params } }

    pub const fn params(&self) -> &DncParams { &self.params }

    fn convert(&self, raw_file: &Path) -> Result<PathBuf, DecodeError> {
        let raw_file = std::path::absolute(raw_file).map_err(|error| {
            DecodeError::InvalidInput(
                format!("cannot resolve RAW path {}: {error}", raw_file.display()).into(),
            )
        })?;
        if !raw_file.is_file() {
            return Err(DecodeError::InvalidInput(
                format!("RAW input is not a file: {}", raw_file.display()).into(),
            ));
        }

        if raw_file
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dng"))
        {
            clerk::info!("The input file is dng.");
            return Ok(raw_file);
        }

        let dng_file = self.params.output_file(&raw_file)?;
        let output_directory = dng_file.parent().ok_or_else(|| {
            DecodeError::InvalidInput(
                format!("DNG output path has no parent: {}", dng_file.display()).into(),
            )
        })?;
        std::fs::create_dir_all(output_directory).map_err(|error| {
            DecodeError::Backend(
                format!(
                    "failed to create DNG output directory {}: {error}",
                    output_directory.display()
                )
                .into(),
            )
        })?;

        if self.params.overwrite {
            match std::fs::remove_file(&dng_file) {
                Ok(()) => {
                    clerk::info!(
                        "Remove(overwrite) existing dng file: {}",
                        dng_file.display()
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(DecodeError::Backend(
                        format!(
                            "failed to remove existing DNG {}: {error}",
                            dng_file.display()
                        )
                        .into(),
                    ));
                }
            }
        }

        if dng_file.exists() {
            let dng_file_absolute = std::path::absolute(&dng_file).unwrap();
            clerk::info!("DNG file already exists: {}", dng_file_absolute.display());
        } else {
            let program = &*DNC_EXECUTABLE;
            let args = self.params.to_cmd(&raw_file)?;
            let output = std::process::Command::new(program)
                .args(&args)
                .output()
                .map_err(|error| {
                    DecodeError::Backend(
                        format!(
                            "failed to launch {} for {}: {error}",
                            program.display(),
                            raw_file.display()
                        )
                        .into(),
                    )
                })?;

            clerk::debug!("Command:\n{} {}", program.to_str().unwrap(), args.join(" "));
            clerk::debug!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            clerk::debug!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            if !output.status.success() {
                return Err(DecodeError::Backend(
                    format!(
                        "{} failed with {}: {}",
                        program.display(),
                        output.status,
                        String::from_utf8_lossy(&output.stderr).trim()
                    )
                    .into(),
                ));
            }
            if !dng_file.exists() {
                return Err(DecodeError::Backend(
                    format!(
                        "{} completed without creating {}",
                        program.display(),
                        dng_file.display()
                    )
                    .into(),
                ));
            }
            let dng_file_absolute = std::path::absolute(&dng_file).unwrap();
            clerk::debug!("Write dng to: {}", dng_file_absolute.display());
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
        RawlerDecoder.decode_full(&dng_file as &Path)
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        let dng_file = self.convert(input)?;
        RawlerDecoder.decode_active(&dng_file as &Path)
    }
}
