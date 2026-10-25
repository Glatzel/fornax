use std::collections::HashMap;

use image::math::Rect;

use crate::BayerPattern;

pub type ExtraMetadata = HashMap<String, MetadataValue>;

#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(String),
    Bytes(Box<[u8]>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    pub decoder: DecoderMetadata,
    pub preprocess: PreprocessMetadata,
    pub postprocess: PostprocessMetadata,
    pub extra: ExtraMetadata,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecoderMetadata {
    pub backend: String,
    pub revision: Option<String>,
    pub vendor: VendorMetadata,
    pub capture: CaptureMetadata,
}

impl Default for DecoderMetadata {
    fn default() -> Self {
        Self {
            backend: "unknown".into(),
            revision: None,
            vendor: VendorMetadata::default(),
            capture: CaptureMetadata::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorMetadata {
    pub make: Option<String>,
    pub model: Option<String>,
    pub normalized_make: Option<String>,
    pub normalized_model: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaptureMetadata {
    pub software: Option<String>,
    pub timestamp: Option<i64>,
    pub iso_speed: Option<f32>,
    pub shutter: Option<f32>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreprocessMetadata {
    pub active_area: Rect,
    pub cfa: CfaMetadata,
    pub levels: LevelsMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CfaMetadata {
    pub bayer_pattern: BayerPattern,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelsMetadata {
    pub black: BlackLevels,
    pub white: WhiteLevels,
}

impl Default for LevelsMetadata {
    fn default() -> Self {
        Self {
            black: BlackLevels::default(),
            white: WhiteLevels::Uniform(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlackLevels {
    pub common: u32,
    pub per_slot: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WhiteLevels {
    Uniform(u32),
    PerSlot([u32; 4]),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PostprocessMetadata {
    pub white_balance: WhiteBalanceMetadata,
    pub color_matrices: ColorMatrixMetadata,
    pub orientation: Option<u8>,
    pub default_crop: Option<DefaultCrop>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WhiteBalanceMetadata {
    pub as_shot: [f32; 4],
}

impl Default for WhiteBalanceMetadata {
    fn default() -> Self { Self { as_shot: [1.0; 4] } }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorMatrixMetadata {
    pub camera_to_srgb: [[f32; 4]; 3],
    pub camera_to_xyz: [[f32; 3]; 4],
    pub camera_matrix: [[f32; 4]; 3],
    pub color_correction_matrix: [[f32; 4]; 3],
}

impl Default for ColorMatrixMetadata {
    fn default() -> Self {
        Self {
            camera_to_srgb: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            camera_to_xyz: [[0.0; 3]; 4],
            camera_matrix: [[0.0; 4]; 3],
            color_correction_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultCrop {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessedMetadata {
    pub decoder: DecoderMetadata,
    pub demosaic: DemosaicMetadata,
    pub postprocess: PostprocessMetadata,
    pub extra: ExtraMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DemosaicMetadata {
    pub active_area: Rect,
    pub bayer_pattern: BayerPattern,
}
