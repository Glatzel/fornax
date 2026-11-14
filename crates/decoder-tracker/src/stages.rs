use std::fmt::Display;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use fornax_dng::Directory;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_decoder::Decoder;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, Value};

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StageRow {
    pub decode_pass: u8,
    pub raw_mosaic_pass: u8,
    pub raw_sha256: String,
    pub process_metadata_pass: u8,
    pub handoff_pass: u8,
    pub error_kind: String,
    pub width: u32,
    pub height: u32,
    pub cfa: String,
    pub compression_id: Option<u32>,
    pub compression_name: String,
    pub storage: String,
    pub bits_per_sample: Option<u16>,
    pub sample_format: String,
    pub predictor: Option<u16>,
    pub rows_per_strip: Option<u32>,
    pub tile_width: Option<u32>,
    pub tile_height: Option<u32>,
}

impl Default for StageRow {
    fn default() -> Self {
        Self {
            decode_pass: 0,
            raw_mosaic_pass: 0,
            raw_sha256: String::new(),
            process_metadata_pass: 0,
            handoff_pass: 0,
            error_kind: String::new(),
            width: 0,
            height: 0,
            cfa: "unknown".to_owned(),
            compression_id: None,
            compression_name: "unknown".to_owned(),
            storage: "unknown".to_owned(),
            bits_per_sample: None,
            sample_format: String::new(),
            predictor: None,
            rows_per_strip: None,
            tile_width: None,
            tile_height: None,
        }
    }
}

impl StageRow {
    /// Attach the source encoding selected by a constructed decoder.
    pub fn with_encoding(mut self, encoding: EncodingMetadata) -> Self {
        self.compression_id = encoding.compression_id;
        self.compression_name = encoding.compression_name.to_owned();
        self.storage = encoding.storage.to_owned();
        self.bits_per_sample = encoding.bits_per_sample;
        self.sample_format = encoding
            .sample_format
            .map_or_else(String::new, |format| format.as_str().to_owned());
        self.predictor = encoding.predictor;
        self.rows_per_strip = encoding.rows_per_strip;
        self.tile_width = encoding.tile_width;
        self.tile_height = encoding.tile_height;
        self
    }
}

pub fn transition_row<E>(
    decode: Result<(), E>,
    raw_mosaic: Result<(), E>,
    process_metadata: Result<(), E>,
    handoff: Result<(), E>,
) -> StageRow
where
    E: Display,
{
    let stages = [decode, raw_mosaic, process_metadata, handoff];
    let mut row = StageRow::default();
    for (index, stage) in stages.into_iter().enumerate() {
        match stage {
            Ok(()) => match index {
                0 => row.decode_pass = 1,
                1 => row.raw_mosaic_pass = 1,
                2 => row.process_metadata_pass = 1,
                3 => row.handoff_pass = 1,
                _ => unreachable!("there are four stages"),
            },
            Err(error) => {
                row.error_kind = error.to_string();
                break;
            }
        }
    }
    row
}

pub const fn classify_decode_error(error: &DecodeError) -> &'static str {
    match error {
        DecodeError::InvalidInput(_) => "invalid_input",
        DecodeError::Unsupported(_) => "unsupported",
        DecodeError::Backend(_) => "backend",
        DecodeError::InvalidMetadata(_) => "invalid_metadata",
        DecodeError::InvalidOutput(_) => "invalid_output",
        DecodeError::Other(_) => "other",
    }
}

pub fn run_guarded<F>(operation: F) -> StageRow
where
    F: FnOnce() -> std::result::Result<StageRow, String>,
{
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(row)) => row,
        Ok(Err(error)) => runner_row(&error),
        Err(_) => runner_row("runner:panic"),
    }
}

pub fn runner_row(error_kind: &str) -> StageRow {
    StageRow {
        error_kind: error_kind.to_owned(),
        ..StageRow::default()
    }
}

pub fn evaluate_decoder<D>(path: &Path) -> std::result::Result<StageRow, String>
where
    D: IDecoder + IDecoderDng + IDecoderEncoding,
{
    clerk::info!("evaluating {}", path.display());
    let decoder =
        D::decode(path).map_err(|error| format!("decode:{}", classify_decode_error(&error)))?;
    let encoding = decoder.encoding_metadata();
    let raw = match decoder.raw_mosaic() {
        Ok(raw) => raw,
        Err(error) => {
            return Ok(StageRow {
                error_kind: format!("raw_mosaic:{}", classify_decode_error(&error)),
                ..StageRow::default()
            }
            .with_encoding(encoding));
        }
    };
    let (width, height) = raw.dimensions();
    if width == 0 || height == 0 || !sample_count_matches(&raw) {
        return Ok(StageRow {
            decode_pass: 1,
            error_kind: "raw_mosaic:bad_dimensions".to_owned(),
            width,
            height,
            ..StageRow::default()
        }
        .with_encoding(encoding));
    }
    let raw_sha256 = crate::golden::raw_sha256(&raw);
    let dng = match decoder.dng_metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(StageRow {
                decode_pass: 1,
                raw_mosaic_pass: 1,
                raw_sha256,
                error_kind: format!("process_metadata:{}", classify_decode_error(&error)),
                width,
                height,
                ..StageRow::default()
            }
            .with_encoding(encoding));
        }
    };
    let cfa = describe_cfa(&dng);
    let process = match decoder.process_metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(StageRow {
                decode_pass: 1,
                raw_mosaic_pass: 1,
                raw_sha256,
                error_kind: format!("process_metadata:{}", classify_decode_error(&error)),
                width,
                height,
                cfa,
                ..StageRow::default()
            }
            .with_encoding(encoding));
        }
    };
    if let Err(error) = sanity_check_metadata(width, height, &dng, &process) {
        return Ok(StageRow {
            decode_pass: 1,
            raw_mosaic_pass: 1,
            raw_sha256,
            error_kind: error,
            width,
            height,
            cfa,
            ..StageRow::default()
        }
        .with_encoding(encoding));
    }
    let handoff = match decoder.cropped_mosaic::<f32>() {
        Ok(cropped) => {
            let (cropped_width, cropped_height) = cropped.image().dimensions();
            if cropped_width == process.active_area.width
                && cropped_height == process.active_area.height
            {
                Ok(())
            } else {
                eprintln!(
                    "handoff dimensions {cropped_width}x{cropped_height} do not match active area {}x{}",
                    process.active_area.width, process.active_area.height
                );
                Err("handoff:bad_dimensions")
            }
        }
        Err(error) => Err(match classify_decode_error(&error) {
            "invalid_input" => "handoff:invalid_input",
            "unsupported" => "handoff:unsupported",
            "backend" => "handoff:backend",
            "invalid_metadata" => "handoff:invalid_metadata",
            "invalid_output" => "handoff:invalid_output",
            _ => "handoff:other",
        }),
    };
    Ok(StageRow {
        decode_pass: 1,
        raw_mosaic_pass: 1,
        raw_sha256,
        process_metadata_pass: 1,
        handoff_pass: u8::from(handoff.is_ok()),
        error_kind: handoff.err().unwrap_or_default().to_owned(),
        width,
        height,
        cfa,
        ..StageRow::default()
    }
    .with_encoding(encoding))
}

pub fn evaluate_one(path: &Path) -> StageRow { run_guarded(|| evaluate_decoder::<Decoder>(path)) }

fn sample_count_matches(raw: &RawMosaic) -> bool {
    let (width, height) = raw.dimensions();
    let Some(expected) = (width as usize).checked_mul(height as usize) else {
        return false;
    };
    let actual = match raw {
        RawMosaic::U8(image) => image.as_raw().len(),
        RawMosaic::U16(image) => image.as_raw().len(),
        RawMosaic::U32(image) => image.as_raw().len(),
        RawMosaic::F32(image) => image.as_raw().len(),
    };
    actual == expected
}

pub fn sanity_check_metadata(
    full_width: u32,
    full_height: u32,
    dng: &Directory,
    process: &ProcessMetadata,
) -> std::result::Result<(), String> {
    let image_width = required_u32(dng, TiffTag::ImageWidth)
        .map_err(|()| "process_metadata:bad_geometry".to_owned())?;
    let image_height = required_u32(dng, TiffTag::ImageLength)
        .map_err(|()| "process_metadata:bad_geometry".to_owned())?;
    if image_width != full_width || image_height != full_height {
        return Err("process_metadata:bad_geometry".to_owned());
    }
    let active = process.active_area;
    if active.width == 0
        || active.height == 0
        || active
            .x
            .checked_add(active.width)
            .is_none_or(|right| right > full_width)
        || active
            .y
            .checked_add(active.height)
            .is_none_or(|bottom| bottom > full_height)
    {
        return Err("process_metadata:bad_geometry".to_owned());
    }
    let crop = process.default_crop;
    if crop.width == 0
        || crop.height == 0
        || crop
            .x
            .checked_add(crop.width)
            .is_none_or(|right| right > active.width)
        || crop
            .y
            .checked_add(crop.height)
            .is_none_or(|bottom| bottom > active.height)
    {
        return Err("process_metadata:bad_geometry".to_owned());
    }
    if !process.levels.white.is_finite() || process.levels.white <= 0.0 {
        return Err("process_metadata:bad_levels".to_owned());
    }
    let black = &process.levels.black;
    let (black_width, black_height) = black.dimensions();
    if black_width == 0
        || black_height == 0
        || black.as_raw().len() != black_width as usize * black_height as usize
        || black
            .as_raw()
            .iter()
            .any(|value| !value.is_finite() || *value >= process.levels.white)
    {
        return Err("process_metadata:bad_levels".to_owned());
    }
    if (!process.levels.black_delta_h.is_empty()
        && process.levels.black_delta_h.len() != full_width as usize)
        || (!process.levels.black_delta_v.is_empty()
            && process.levels.black_delta_v.len() != full_height as usize)
        || process
            .levels
            .black_delta_h
            .iter()
            .chain(&process.levels.black_delta_v)
            .any(|value| !value.is_finite())
    {
        return Err("process_metadata:bad_levels".to_owned());
    }
    Ok(())
}

pub fn describe_cfa(metadata: &Directory) -> String {
    let Some(repeat) = values(metadata.get(&Tag::from(TiffTag::CfaRepeatPatternDim))) else {
        return "unknown".to_owned();
    };
    let Some([width, height]) = as_pair(&repeat) else {
        return "unknown".to_owned();
    };
    let Some(pattern) = values(metadata.get(&Tag::from(TiffTag::CfaPattern))) else {
        return "unknown".to_owned();
    };
    if pattern.len() != width as usize * height as usize {
        return format!("CFA-{width}x{height}");
    }
    if width == 6 && height == 6 {
        return "XTrans-6x6".to_owned();
    }
    if width == 2 && height == 2 {
        return pattern
            .iter()
            .map(|value| match *value {
                0 => "R",
                1 => "G",
                2 => "B",
                _ => "?",
            })
            .collect();
    }
    format!("CFA-{width}x{height}")
}

fn required_u32(metadata: &Directory, tag: TiffTag) -> std::result::Result<u32, ()> {
    values(metadata.get(&Tag::from(tag)))
        .and_then(|values| values.first().copied())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(())
}

fn as_pair(values: &[u64]) -> Option<[u32; 2]> {
    let [width, height] = values else {
        return None;
    };
    Some([u32::try_from(*width).ok()?, u32::try_from(*height).ok()?])
}

fn values(value: Option<&Value>) -> Option<Vec<u64>> {
    match value? {
        Value::List(values) => values.iter().map(scalar).collect(),
        value => Some(vec![scalar(value)?]),
    }
}

fn scalar(value: &Value) -> Option<u64> {
    Some(match value {
        Value::Byte(value) => u64::from(*value),
        Value::Short(value) => u64::from(*value),
        Value::Unsigned(value) | Value::Ifd(value) => u64::from(*value),
        Value::UnsignedBig(value) | Value::IfdBig(value) => *value,
        Value::SignedByte(value) => u64::try_from(*value).ok()?,
        Value::SignedShort(value) => u64::try_from(*value).ok()?,
        Value::Signed(value) => u64::try_from(*value).ok()?,
        Value::SignedBig(value) => u64::try_from(*value).ok()?,
        _ => return None,
    })
}
