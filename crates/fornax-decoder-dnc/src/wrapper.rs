use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use fornax_core::error::DecodeError;
use fornax_core::metadata::Metadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{DecodedRawActive, DecodedRawFull, Primitive};
use fornax_decoder_rawler::RawlerDecoder;
use strum::{AsRefStr, Display, EnumString};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, AsRefStr, EnumString, Display,
)]
pub enum DncPreview {
    #[cfg_attr(feature = "serde", serde(rename = "-p0"))]
    #[strum(serialize = "-p0")]
    None,
    #[cfg_attr(feature = "serde", serde(rename = "-p1"))]
    #[strum(serialize = "-p1")]
    Medium,
    #[cfg_attr(feature = "serde", serde(rename = "-p2"))]
    #[strum(serialize = "-p2")]
    Full,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, AsRefStr, EnumString, Display,
)]
pub enum DncCompatibility {
    #[cfg_attr(feature = "serde", serde(rename = "-cr2.4"))]
    #[strum(serialize = "-cr2.4")]
    CR2_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr4.1"))]
    #[strum(serialize = "-cr4.1")]
    CR4_1,
    #[cfg_attr(feature = "serde", serde(rename = "-cr4.6"))]
    #[strum(serialize = "-cr4.6")]
    CR4_6,
    #[cfg_attr(feature = "serde", serde(rename = "-cr5.4"))]
    #[strum(serialize = "-cr5.4")]
    CR5_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr6.6"))]
    #[strum(serialize = "-cr6.6")]
    CR6_6,
    #[cfg_attr(feature = "serde", serde(rename = "-cr7.1"))]
    #[strum(serialize = "-cr7.1")]
    CR7_1,
    #[cfg_attr(feature = "serde", serde(rename = "-cr11.2"))]
    #[strum(serialize = "-cr11.2")]
    CR11_2,
    #[cfg_attr(feature = "serde", serde(rename = "-cr12.4"))]
    #[strum(serialize = "-cr12.4")]
    CR12_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr13.2"))]
    #[strum(serialize = "-cr13.2")]
    CR13_2,
    #[cfg_attr(feature = "serde", serde(rename = "-cr14.0"))]
    #[strum(serialize = "-cr14.0")]
    CR14_0,
    #[cfg_attr(feature = "serde", serde(rename = "-cr15.3"))]
    #[strum(serialize = "-cr15.3")]
    CR15_3,
    #[cfg_attr(feature = "serde", serde(rename = "-cr16.0"))]
    #[strum(serialize = "-cr16.0")]
    CR16_0,

    #[cfg_attr(feature = "serde", serde(rename = "-dng1.1"))]
    #[strum(serialize = "-dng1.1")]
    DNG1_1,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.3"))]
    #[strum(serialize = "-dng1.3")]
    DNG1_3,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.4"))]
    #[strum(serialize = "-dng1.4")]
    DNG1_4,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.5"))]
    #[strum(serialize = "-dng1.5")]
    DNG1_5,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.6"))]
    #[strum(serialize = "-dng1.6")]
    DNG1_6,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.7"))]
    #[strum(serialize = "-dng1.7")]
    DNG1_7,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.7.1"))]
    #[strum(serialize = "-dng1.7.1")]
    DNG1_7_1,
}

static DNC_EXECUTABLE: LazyLock<PathBuf> = LazyLock::new(|| {
    #[cfg(target_os = "macos")]
    let executable = "Adobe DNG Converter";
    #[cfg(target_os = "windows")]
    let executable = "Adobe DNG Converter.exe";

    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(executable))
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DncDecoder {
    /// Output lossless compressed DNG files.
    pub compressed: bool,
    /// Output linear DNG files.
    pub linear: bool,
    /// Embed original raw file inside DNG files.
    pub embed: bool,
    /// Set JPEG preview size.
    pub preview: DncPreview,
    /// Embed fast load data inside DNG files.
    pub fast_load: bool,
    /// Limit size to `num` pixels per side.
    pub side: Option<u32>,
    /// Limit pixel count to `num` pixels per image.
    pub count: Option<u32>,
    /// Set Camera Raw compatibility.
    pub compatibility: DncCompatibility,
    /// Output converted files to the specified directory.
    ///
    /// Defaults to the input file's directory.
    pub directory: Option<PathBuf>,
    /// Output filename.
    ///
    /// Defaults to the input stem with a `.dng` extension.
    pub filename: Option<String>,
    /// Overwrite an existing DNG file.
    pub overwrite: bool,
}
impl DncDecoder {
    fn command_args(&self, raw_file: &Path) -> Result<Vec<String>, DecodeError> {
        let raw_file = std::path::absolute(raw_file).map_err(|error| {
            DecodeError::InvalidInput(
                format!("cannot resolve RAW path {}: {error}", raw_file.display()).into(),
            )
        })?;
        let output_file = self.output_file(&raw_file)?;
        let directory = output_file.parent().ok_or_else(|| {
            DecodeError::InvalidInput(
                format!("output path has no parent: {}", output_file.display()).into(),
            )
        })?;
        let filename = output_file.file_name().ok_or_else(|| {
            DecodeError::InvalidInput(
                format!("output path has no filename: {}", output_file.display()).into(),
            )
        })?;

        let mut cmd = Vec::new();
        cmd.push(if self.compressed { "-c" } else { "-u" }.into());
        if self.linear {
            cmd.push("-l".into());
        }
        cmd.push(self.preview.to_string());
        if self.fast_load {
            cmd.push("-fl".into());
        }
        if let Some(side) = self.side {
            cmd.push("-side".into());
            cmd.push(side.to_string());
        }
        if let Some(count) = self.count {
            cmd.push("-count".into());
            cmd.push(count.to_string());
        }
        cmd.push(self.compatibility.to_string());
        cmd.push("-d".into());
        cmd.push(directory.to_string_lossy().into());
        cmd.push("-o".into());
        cmd.push(filename.to_string_lossy().into());
        cmd.push(raw_file.to_string_lossy().into());
        Ok(cmd)
    }

    fn output_file(&self, raw_file: &Path) -> Result<PathBuf, DecodeError> {
        let mut directory = match &self.directory {
            None => raw_file
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
            Some(directory) => std::path::absolute(directory).map_err(|error| {
                DecodeError::InvalidInput(
                    format!(
                        "cannot resolve DNC output directory {}: {error}",
                        directory.display()
                    )
                    .into(),
                )
            })?,
        };

        let filename = match &self.filename {
            Some(filename) if filename.is_empty() => {
                return Err(DecodeError::InvalidInput(
                    "DNC output filename must not be empty".into(),
                ));
            }
            Some(filename) => PathBuf::from(filename),
            None => PathBuf::from(
                raw_file
                    .file_stem()
                    .map(|stem| {
                        let mut filename = OsString::from(stem);
                        filename.push(".dng");
                        filename
                    })
                    .ok_or_else(|| {
                        DecodeError::InvalidInput(
                            format!("RAW path has no filename: {}", raw_file.display()).into(),
                        )
                    })?,
            ),
        };

        directory.push(filename);
        Ok(directory)
    }

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

        let dng_file = self.output_file(&raw_file)?;
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

        if self.overwrite {
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
            let args = self.command_args(&raw_file)?;
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
    fn default() -> Self {
        Self {
            compressed: true,
            linear: false,
            embed: false,
            preview: DncPreview::Medium,
            fast_load: false,
            side: None,
            count: None,
            compatibility: DncCompatibility::CR16_0,
            directory: None,
            filename: None,
            overwrite: false,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_matches_adobe_converter_defaults() {
        let decoder = DncDecoder::default();
        assert!(decoder.compressed);
        assert_eq!(decoder.preview, DncPreview::Medium);
        assert_eq!(decoder.compatibility, DncCompatibility::CR16_0);
        assert!(!decoder.linear);
        assert!(!decoder.overwrite);
    }

    #[test]
    fn command_arguments_preserve_paths_and_separate_values() -> Result<(), DecodeError> {
        let raw = std::env::temp_dir().join("fornax input.CR2");
        let decoder = DncDecoder {
            side: Some(4_000),
            count: Some(20_000_000),
            ..DncDecoder::default()
        };
        let args = decoder.command_args(&raw)?;

        assert!(args.windows(2).any(|args| args == ["-side", "4000"]));
        assert!(args.windows(2).any(|args| args == ["-count", "20000000"]));
        assert!(args.iter().any(|arg| arg == "fornax input.dng"));
        assert!(args.iter().any(|arg| arg.ends_with("fornax input.CR2")));
        Ok(())
    }

    #[test]
    fn empty_output_filename_is_rejected() {
        let decoder = DncDecoder {
            filename: Some(String::new()),
            ..DncDecoder::default()
        };
        let error = decoder
            .command_args(Path::new("input.cr2"))
            .expect_err("empty filename must fail");
        assert!(matches!(error, DecodeError::InvalidInput(_)));
    }
}
