use core::fmt::Display;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fornax_core::error::DecodeError;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DncPreview {
    #[cfg_attr(feature = "serde", serde(rename = "-p0"))]
    None,
    #[cfg_attr(feature = "serde", serde(rename = "-p1"))]
    Medium,
    #[cfg_attr(feature = "serde", serde(rename = "-p2"))]
    Full,
}
impl Display for DncPreview {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::None => "-p0",
            Self::Medium => "-p1",
            Self::Full => "-p2",
        };
        write!(f, "{text}")
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DncCompatibility {
    #[cfg_attr(feature = "serde", serde(rename = "-cr2.4"))]
    CR2_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr4.1"))]
    CR4_1,
    #[cfg_attr(feature = "serde", serde(rename = "-cr4.6"))]
    CR4_6,
    #[cfg_attr(feature = "serde", serde(rename = "-cr5.4"))]
    CR5_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr6.6"))]
    CR6_6,
    #[cfg_attr(feature = "serde", serde(rename = "-cr7.1"))]
    CR7_1,
    #[cfg_attr(feature = "serde", serde(rename = "-cr11.2"))]
    CR11_2,
    #[cfg_attr(feature = "serde", serde(rename = "-cr12.4"))]
    CR12_4,
    #[cfg_attr(feature = "serde", serde(rename = "-cr13.2"))]
    CR13_2,
    #[cfg_attr(feature = "serde", serde(rename = "-cr14.0"))]
    CR14_0,
    #[cfg_attr(feature = "serde", serde(rename = "-cr15.3"))]
    CR15_3,
    #[cfg_attr(feature = "serde", serde(rename = "-cr16.0"))]
    CR16_0,

    #[cfg_attr(feature = "serde", serde(rename = "-dng1.1"))]
    DNG1_1,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.3"))]
    DNG1_3,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.4"))]
    DNG1_4,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.5"))]
    DNG1_5,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.6"))]
    DNG1_6,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.7"))]
    DNG1_7,
    #[cfg_attr(feature = "serde", serde(rename = "-dng1.7.1"))]
    DNG1_7_1,
}
impl Display for DncCompatibility {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::CR2_4 => "-cr2.4",
            Self::CR4_1 => "-cr4.1",
            Self::CR4_6 => "-cr4.6",
            Self::CR5_4 => "-cr5.4",
            Self::CR6_6 => "-cr6.6",
            Self::CR7_1 => "-cr7.1",
            Self::CR11_2 => "-cr11.2",
            Self::CR12_4 => "-cr12.4",
            Self::CR13_2 => "-cr13.2",
            Self::CR14_0 => "-cr14.0",
            Self::CR15_3 => "-cr15.3",
            Self::CR16_0 => "-cr16.0",
            Self::DNG1_1 => "-dng1.1",
            Self::DNG1_3 => "-dng1.3",
            Self::DNG1_4 => "-dng1.4",
            Self::DNG1_5 => "-dng1.5",
            Self::DNG1_6 => "-dng1.6",
            Self::DNG1_7 => "-dng1.7",
            Self::DNG1_7_1 => "-dng1.7.1",
        };
        write!(f, "{text}")
    }
}
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

        cmd.push(self.preview.to_string().into());

        if self.fast_load {
            cmd.push("-fl".into());
        }

        if let Some(side) = self.side {
            cmd.push("-side".into());
            cmd.push(side.to_string().into());
        }

        if let Some(count) = self.count {
            cmd.push("-count".into());
            cmd.push(count.to_string().into());
        }
        cmd.push(self.compatibility.to_string().into());

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
