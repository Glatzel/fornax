use std::fmt::Display;
use std::path::PathBuf;

use path_slash::PathBufExt;
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Copy, Clone)]
pub enum DngConverterPreview {
    None,
    Medium,
    Full,
}
impl Display for DngConverterPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            DngConverterPreview::None => "-p0",
            DngConverterPreview::Medium => "-p1",
            DngConverterPreview::Full => "-p2",
        };
        write!(f, "{}", text)
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Copy, Clone)]
pub enum DngConverterCompatibility {
    CR2_4,
    CR4_1,
    CR4_6,
    CR5_4,
    CR6_6,
    CR7_1,
    CR11_2,
    CR12_4,
    CR13_2,
    CR14_0,
    CR15_3,
    CR16_0,

    DNG1_1,
    DNG1_3,
    DNG1_4,
    DNG1_5,
    DNG1_6,
    DNG1_7,
    DNG1_7_1,
}
impl Display for DngConverterCompatibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            DngConverterCompatibility::CR2_4 => "-cr2.4",
            DngConverterCompatibility::CR4_1 => "-cr4.1",
            DngConverterCompatibility::CR4_6 => "-cr4.6",
            DngConverterCompatibility::CR5_4 => "-cr5.4",
            DngConverterCompatibility::CR6_6 => "-cr6.6",
            DngConverterCompatibility::CR7_1 => "-cr7.1",
            DngConverterCompatibility::CR11_2 => "-cr11.2",
            DngConverterCompatibility::CR12_4 => "-cr12.4",
            DngConverterCompatibility::CR13_2 => "-cr13.2",
            DngConverterCompatibility::CR14_0 => "-cr14.0",
            DngConverterCompatibility::CR15_3 => "-cr15.3",
            DngConverterCompatibility::CR16_0 => "-cr16.0",
            DngConverterCompatibility::DNG1_1 => "-dng1.1",
            DngConverterCompatibility::DNG1_3 => "-dng1.3",
            DngConverterCompatibility::DNG1_4 => "-dng1.4",
            DngConverterCompatibility::DNG1_5 => "-dng1.5",
            DngConverterCompatibility::DNG1_6 => "-dng1.6",
            DngConverterCompatibility::DNG1_7 => "-dng1.7",
            DngConverterCompatibility::DNG1_7_1 => "-dng1.7.1",
        };
        write!(f, "{}", text)
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone)]
pub struct DngConverterParams {
    ///Output lossless compressed DNG files
    pub compressed: bool,
    /// Output linear DNG files.
    pub linear: bool,
    ///Embed original raw file inside DNG files.
    pub embed: bool,
    ///Set JPEG preview size.
    pub preview: DngConverterPreview,
    ///Embed fast load data inside DNG files.
    pub fast_load: bool,
    ///Limit size to <num> pixels/side.
    pub side: Option<u32>,
    ///Limit pixel count to <num> pixels/image.
    pub count: Option<u32>,
    ///Limit pixel count to <num> pixels/image.
    pub compatibility: DngConverterCompatibility,
    ///Output converted files to the specified directory.  
    ///
    ///Default is the same directory as the input file.
    pub directory: Option<PathBuf>,
    ///Specify the name of the output DNG file.  
    ///
    ///Default is the name of the input file with the extension  
    ///changed to “.dng”.
    pub filename: Option<String>,
}
impl DngConverterParams {
    pub fn to_cmd(&self, raw_file: &PathBuf) -> Vec<String> {
        let mut cmd: Vec<String> = Vec::new();

        if self.compressed {
            cmd.push("-c".to_string());
        } else {
            cmd.push("-u".to_string());
        }

        if self.linear {
            cmd.push("-l".to_string());
        }

        cmd.push(self.preview.to_string());

        if self.fast_load {
            cmd.push("-fl".to_string());
        }

        if let Some(side) = self.side {
            cmd.push(format!("-side {}", side));
        }

        if let Some(count) = self.count {
            cmd.push(format!("-count {}", count));
        }
        cmd.push(self.compatibility.to_string());

        cmd.push("-d".to_string());
        if let Some(directory) = &self.directory {
            cmd.push(
                dunce::canonicalize(directory)
                    .unwrap()
                    .to_slash_lossy()
                    .to_string(),
            );
        } else {
            cmd.push(
                dunce::canonicalize(raw_file.parent().unwrap())
                    .unwrap()
                    .to_slash_lossy()
                    .to_string(),
            );
        }

        cmd.push("-o".to_string());
        if let Some(filename) = &self.filename {
            cmd.push(filename.clone());
        } else {
            cmd.push(raw_file.file_stem().unwrap().to_string_lossy().to_string());
        }
        cmd.push(raw_file.to_slash_lossy().to_string());
        cmd
    }
}
impl Default for DngConverterParams {
    fn default() -> Self {
        Self {
            compressed: true,
            linear: false,
            embed: false,
            preview: DngConverterPreview::Medium,
            fast_load: false,
            side: None,
            count: None,
            compatibility: DngConverterCompatibility::CR16_0,
            directory: None,
            filename: None,
        }
    }
}
