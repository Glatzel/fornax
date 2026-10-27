use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fornax_core::error::DecodeError;
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
    #[strum(serialize = "-p2")]
    #[cfg_attr(feature = "serde", serde(rename = "-p2"))]
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
    #[strum(serialize = "15.3")]
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

///# References
/// [dng_converter_commandline](https://helpx.adobe.com/content/dam/help/en/camera-raw/digital-negative/jcr_content/root/content/flex/items/position/position-par/download_section/download-1/dng_converter_commandline.pdf)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DncParams {
    /// Output lossless compressed DNG files
    pub compressed: bool,
    /// Output linear DNG files.
    pub linear: bool,
    /// Embed original raw file inside DNG files.
    pub embed: bool,
    /// Set JPEG preview size.
    pub preview: DncPreview,
    /// Embed fast load data inside DNG files.
    pub fast_load: bool,
    /// Limit size to `num` pixels/side.
    pub side: Option<u32>,
    /// Limit pixel count to `num` pixels/image.
    pub count: Option<u32>,
    /// Set Camera Raw compatibility.
    pub compatibility: DncCompatibility,
    /// Output converted files to the specified directory.
    ///
    /// Default is the same directory as the input file.
    pub directory: Option<PathBuf>,
    /// Specify the name of the output DNG file.
    ///
    /// Default is the name of the input file with the extension
    /// changed to “.dng”.
    pub filename: Option<String>,
    // extra option
    /// Overwrite existing dng file.
    pub overwrite: bool,
}
impl DncParams {
    pub fn to_cmd(&self, raw_file: &Path) -> Result<Vec<String>, DecodeError> {
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

        let mut cmd: Vec<String> = Vec::new();

        if self.compressed {
            cmd.push("-c".into());
        } else {
            cmd.push("-u".into());
        }

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

    pub(crate) fn output_file(&self, raw_file: &Path) -> Result<PathBuf, DecodeError> {
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
}
impl Default for DncParams {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_arguments_preserve_paths_and_separate_values() -> Result<(), DecodeError> {
        let raw = std::env::temp_dir().join("fornax input.CR2");
        let params = DncParams {
            side: Some(4_000),
            count: Some(20_000_000),
            ..DncParams::default()
        };
        let args = params.to_cmd(&raw)?;

        assert!(args.windows(2).any(|args| args == ["-side", "4000"]));
        assert!(args.windows(2).any(|args| args == ["-count", "20000000"]));
        assert!(args.iter().any(|arg| arg == "fornax input.dng"));
        assert!(args.iter().any(|arg| arg.ends_with("fornax input.CR2")));
        Ok(())
    }

    #[test]
    fn empty_output_filename_is_rejected() {
        let params = DncParams {
            filename: Some(String::new()),
            ..DncParams::default()
        };
        let error = params
            .to_cmd(Path::new("input.cr2"))
            .expect_err("empty filename must fail");
        assert!(matches!(error, DecodeError::InvalidInput(_)));
    }
}
