mod native;

use std::collections::HashMap;
use std::path::Path;

use generic_num::num;
use image::math::Rect;
use native::{
    CaptureData, ColorData, ImageParameters, ImageSizes, LibRawContext, LibRawError,
    LibRawErrorCode, UnpackedRawImage, UnpackedRawImageKind,
};

use super::IDecoder;
use crate::error::{DecodeError, RawValidationError};
use crate::metadata::{
    BlackLevels, CaptureMetadata, CfaMetadata, ColorMatrixMetadata, DecoderMetadata, DefaultCrop,
    ExtraMetadata, LevelsMetadata, Metadata, MetadataValue, PostprocessMetadata,
    PreprocessMetadata, VendorMetadata, WhiteBalanceMetadata, WhiteLevels,
};
use crate::{BayerPattern, DecodedRaw, MonoImage, Primitive};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LibRawDecoder;

impl LibRawDecoder {
    pub const fn new() -> Self { Self }
}

impl<T> IDecoder<Path, T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode(&self, input: &Path) -> Result<DecodedRaw<T>, DecodeError> {
        let mut context = LibRawContext::new().map_err(translate_libraw_error)?;
        context.open_file(input).map_err(translate_libraw_error)?;
        context.unpack().map_err(translate_libraw_error)?;
        decode_context(&context)
    }
}

impl<T> IDecoder<[u8], T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode(&self, input: &[u8]) -> Result<DecodedRaw<T>, DecodeError> {
        let mut context = LibRawContext::new().map_err(translate_libraw_error)?;
        context.open_buffer(input).map_err(translate_libraw_error)?;
        context.unpack().map_err(translate_libraw_error)?;
        decode_context(&context)
    }
}

fn decode_context<T>(context: &LibRawContext) -> Result<DecodedRaw<T>, DecodeError>
where
    T: Primitive,
{
    let sizes = context.image_sizes();
    let parameters = context.parameters().map_err(translate_libraw_error)?;
    let color = context.color_data();
    let capture = context.capture_data();
    validate_layout(&parameters)?;

    let pattern = bayer_pattern(parameters.filters)?;
    let image = raw_image(context, &sizes)?;
    let metadata = metadata(context, &sizes, &parameters, &color, capture, pattern)?;
    Ok(DecodedRaw::new(image, metadata))
}

fn validate_layout(parameters: &ImageParameters) -> Result<(), DecodeError> {
    if parameters.raw_count != 1 {
        return Err(DecodeError::Unsupported(
            format!(
                "multishot RAW count {} is unsupported",
                parameters.raw_count
            )
            .into(),
        ));
    }
    if parameters.is_foveon {
        return Err(DecodeError::Unsupported(
            "Foveon sensors are unsupported".into(),
        ));
    }
    if parameters.colors != 3 || !parameters.cdesc.starts_with("RGBG") {
        return Err(DecodeError::Unsupported(
            format!(
                "non-RGB CFA description is unsupported: colors={}, cdesc={}",
                parameters.colors, parameters.cdesc
            )
            .into(),
        ));
    }
    Ok(())
}

fn translate_libraw_error(error: LibRawError) -> DecodeError {
    match error {
        LibRawError::Native { code, message } => match code {
            LibRawErrorCode::FileUnsupported
            | LibRawErrorCode::Unspecified
            | LibRawErrorCode::NotImplemented => DecodeError::Unsupported(message.into()),
            LibRawErrorCode::DataError
            | LibRawErrorCode::IoError
            | LibRawErrorCode::InputClosed => DecodeError::InvalidInput(message.into()),
            _ => DecodeError::Backend(message.into()),
        },
        LibRawError::InvalidInput(message) => DecodeError::InvalidInput(message.into()),
    }
}

fn bayer_pattern(filters: u32) -> Result<BayerPattern, DecodeError> {
    match filters {
        0 => {
            return Err(DecodeError::Unsupported(
                "non-mosaic RAW is unsupported".into(),
            ));
        }
        1 => {
            return Err(DecodeError::Unsupported("16x16 CFA is unsupported".into()));
        }
        9 => {
            return Err(DecodeError::Unsupported(
                "X-Trans CFA is unsupported".into(),
            ));
        }
        2..=8 | 10..=999 => {
            return Err(DecodeError::Unsupported(
                format!("reserved LibRaw CFA descriptor {filters}").into(),
            ));
        }
        _ => {}
    }

    // LibRaw COLOR(row, column) uses two bits per CFA color.
    let color = |row: u32, col: u32| -> u32 { (filters >> (((row << 1) & 14) + col) * 2) & 3 };

    match (color(0, 0), color(0, 1), color(1, 0), color(1, 1)) {
        (0, 1, 3, 2) => Ok(BayerPattern::RGGB),
        (2, 3, 1, 0) => Ok(BayerPattern::BGGR),
        (1, 0, 2, 3) => Ok(BayerPattern::GRBG),
        (3, 2, 0, 1) => Ok(BayerPattern::GBRG),
        _ => Err(DecodeError::Unsupported(
            format!("unsupported LibRaw CFA pattern {filters:#010x}").into(),
        )),
    }
}

fn active_area(sizes: &ImageSizes) -> Rect {
    Rect {
        x: sizes.left_margin,
        y: sizes.top_margin,
        width: sizes.width,
        height: sizes.height,
    }
}

fn raw_image<T>(context: &LibRawContext, sizes: &ImageSizes) -> Result<MonoImage<T>, DecodeError>
where
    T: Primitive,
{
    let raw = context
        .unpacked_raw_image()
        .map_err(translate_libraw_error)?;
    if raw.has_color3_image()
        || raw.has_color4_image()
        || raw.has_float3_image()
        || raw.has_float4_image()
    {
        return Err(DecodeError::Unsupported(
            "multichannel or LinearRaw payloads are unsupported".into(),
        ));
    }
    let sample_count = checked_sample_count(sizes)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| DecodeError::Backend("could not allocate decoded RAW image".into()))?;

    match raw.kind() {
        UnpackedRawImageKind::Integer => copy_integer_rows(&raw, &mut samples)?,
        UnpackedRawImageKind::Float => copy_float_rows(&raw, &mut samples)?,
        UnpackedRawImageKind::None => {
            return Err(DecodeError::Unsupported(
                "LibRaw did not expose a single-channel raw image".into(),
            ));
        }
    }

    let plane = image::ImageBuffer::from_raw(sizes.raw_width, sizes.raw_height, samples)
        .ok_or(DecodeError::InvalidOutput(RawValidationError::InvalidImage))?;
    Ok(plane)
}

fn copy_integer_rows<T>(raw: &UnpackedRawImage<'_>, samples: &mut Vec<T>) -> Result<(), DecodeError>
where
    T: Primitive,
{
    for row in 0..raw.raw_height() {
        let source = raw
            .integer_row(row)
            .map_err(translate_libraw_error)?
            .ok_or_else(|| DecodeError::Backend("LibRaw raw_image disappeared".into()))?;
        samples.extend(source.iter().map(|sample| num!(*sample)));
    }
    Ok(())
}

fn copy_float_rows<T>(raw: &UnpackedRawImage<'_>, samples: &mut Vec<T>) -> Result<(), DecodeError>
where
    T: Primitive,
{
    for row in 0..raw.raw_height() {
        let source = raw
            .float_row(row)
            .map_err(translate_libraw_error)?
            .ok_or_else(|| DecodeError::Backend("LibRaw float_image disappeared".into()))?;
        samples.extend(source.iter().map(|sample| num!(*sample)));
    }
    Ok(())
}

fn checked_sample_count(sizes: &ImageSizes) -> Result<usize, DecodeError> {
    u64::from(sizes.raw_width)
        .checked_mul(u64::from(sizes.raw_height))
        .and_then(|count| usize::try_from(count).ok())
        .ok_or(DecodeError::InvalidOutput(RawValidationError::InvalidImage))
}

fn metadata(
    context: &LibRawContext,
    sizes: &ImageSizes,
    parameters: &ImageParameters,
    color: &ColorData,
    capture: CaptureData,
    pattern: BayerPattern,
) -> Result<Metadata, DecodeError> {
    let vendor = VendorMetadata {
        make: non_empty(&parameters.make),
        model: non_empty(&parameters.model),
        normalized_make: non_empty(&parameters.normalized_make),
        normalized_model: non_empty(&parameters.normalized_model),
    };
    let capture = CaptureMetadata {
        software: non_empty(&parameters.software),
        timestamp: capture.timestamp,
        iso_speed: positive(capture.iso_speed),
        shutter: positive(capture.shutter),
        aperture: positive(capture.aperture),
        focal_length: positive(capture.focal_length),
    };
    let levels = levels(color)?;
    let matrices = color_matrices(color)?;
    let postprocess = PostprocessMetadata {
        white_balance: WhiteBalanceMetadata {
            as_shot: color.cam_mul,
        },
        color_matrices: matrices,
        orientation: map_orientation(sizes.flip),
        default_crop: map_default_crop(color.default_crop),
    };
    Ok(Metadata {
        decoder: DecoderMetadata {
            backend: "libraw".into(),
            revision: native::version(),
            vendor,
            capture,
        },
        preprocess: PreprocessMetadata {
            active_area: active_area(sizes),
            cfa: CfaMetadata {
                bayer_pattern: pattern,
            },
            levels,
        },
        postprocess,
        extra: extra_metadata(context, sizes, parameters, color)?,
    })
}

fn levels(color: &ColorData) -> Result<LevelsMetadata, DecodeError> {
    if color.cblack[4] != 0 || color.cblack[5] != 0 {
        dbg!(color.black);
        dbg!(color.cblack);
        return Err(DecodeError::Unsupported(
            "spatial LibRaw black-level tiles are unsupported".into(),
        ));
    }
    let black = BlackLevels {
        common: color.black,
        per_slot: [
            color.cblack[0],
            color.cblack[1],
            color.cblack[2],
            color.cblack[3],
        ],
    };
    let white = if color.dng_whitelevel[0] != 0 {
        WhiteLevels::Uniform(color.dng_whitelevel[0])
    } else if color.linear_max.iter().all(|value| *value != 0) {
        WhiteLevels::PerSlot(color.linear_max)
    } else {
        WhiteLevels::Uniform(color.maximum)
    };
    for code in 0..4 {
        let black = f64::from(black.common) + f64::from(black.per_slot[code]);
        let white = match white {
            WhiteLevels::Uniform(white) => f64::from(white),
            WhiteLevels::PerSlot(white) => f64::from(white[code]),
        };
        if white <= black {
            return Err(DecodeError::Backend(
                "LibRaw black and white levels do not form a valid range".into(),
            ));
        }
    }
    Ok(LevelsMetadata { black, white })
}

fn color_matrices(color: &ColorData) -> Result<ColorMatrixMetadata, DecodeError> {
    let values = color
        .cam_mul
        .iter()
        .chain(color.rgb_cam.iter().flatten())
        .chain(color.cam_xyz.iter().flatten())
        .chain(color.cmatrix.iter().flatten())
        .chain(color.ccm.iter().flatten());
    if values.into_iter().any(|value| !value.is_finite()) {
        return Err(DecodeError::Backend(
            "LibRaw returned non-finite color metadata".into(),
        ));
    }
    Ok(ColorMatrixMetadata {
        camera_to_srgb: color.rgb_cam,
        camera_to_xyz: color.cam_xyz,
        camera_matrix: color.cmatrix,
        color_correction_matrix: color.ccm,
    })
}

fn extra_metadata(
    context: &LibRawContext,
    sizes: &ImageSizes,
    parameters: &ImageParameters,
    color: &ColorData,
) -> Result<ExtraMetadata, DecodeError> {
    let mut extra = HashMap::new();
    extra.insert(
        "libraw.maker_index".into(),
        MetadataValue::U64(u64::from(parameters.maker_index)),
    );
    extra.insert(
        "libraw.dng_version".into(),
        MetadataValue::U64(u64::from(parameters.dng_version)),
    );
    extra.insert(
        "libraw.raw_count".into(),
        MetadataValue::U64(u64::from(parameters.raw_count)),
    );
    extra.insert(
        "libraw.raw_bits_per_sample".into(),
        MetadataValue::U64(u64::from(color.raw_bps)),
    );
    extra.insert(
        "libraw.data_maximum".into(),
        MetadataValue::U64(u64::from(color.data_maximum)),
    );
    extra.insert(
        "libraw.raw_pitch".into(),
        MetadataValue::U64(u64::from(sizes.raw_pitch)),
    );
    extra.insert(
        "libraw.flip".into(),
        MetadataValue::I64(i64::from(sizes.flip)),
    );
    extra.insert(
        "libraw.progress_flags".into(),
        MetadataValue::U64(u64::from(context.progress_flags())),
    );
    if let Some(unpack) = context.unpack_function_name() {
        extra.insert("libraw.unpack_function".into(), MetadataValue::Text(unpack));
    }
    Ok(extra)
}

fn map_orientation(flip: i32) -> Option<u8> {
    match flip {
        0 => Some(1),
        3 => Some(3),
        5 => Some(8),
        6 => Some(6),
        _ => None,
    }
}

fn map_default_crop(crop: [u16; 4]) -> Option<DefaultCrop> {
    let [x, y, width, height] = crop;
    (width != 0 && height != 0).then_some(DefaultCrop {
        x,
        y,
        width,
        height,
    })
}

fn non_empty(value: &str) -> Option<String> { (!value.is_empty()).then(|| value.into()) }

fn positive(value: f32) -> Option<f32> { (value.is_finite() && value > 0.0).then_some(value) }

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_filters(tile: [u8; 4]) -> u32 {
        let mut filters = 0_u32;
        for row in 0..8 {
            for column in 0..2 {
                let index = (row * 2 + column) * 2;
                filters |= u32::from(tile[((row & 1) * 2 + column) as usize]) << index;
            }
        }
        filters
    }

    fn color() -> ColorData {
        ColorData {
            black: 10,
            cblack: [1, 2, 3, 4, 0, 0],
            maximum: 110,
            linear_max: [0; 4],
            dng_whitelevel: [0; 4],
            default_crop: [1, 2, 30, 40],
            cam_mul: [2.0, 1.0, 1.5, 1.0],
            rgb_cam: [[1.0; 4]; 3],
            cam_xyz: [[1.0; 3]; 4],
            cmatrix: [[1.0; 4]; 3],
            ccm: [[1.0; 4]; 3],
            raw_bps: 14,
            data_maximum: 100,
        }
    }

    #[test]
    fn recognizes_every_conventional_bayer_phase() {
        for (tile, pattern) in [
            ([0, 1, 3, 2], BayerPattern::RGGB),
            ([2, 3, 1, 0], BayerPattern::BGGR),
            ([1, 0, 2, 3], BayerPattern::GRBG),
            ([3, 2, 0, 1], BayerPattern::GBRG),
        ] {
            assert_eq!(bayer_pattern(encode_filters(tile)).unwrap(), pattern);
        }
    }

    #[test]
    fn rejects_nonstandard_layouts() {
        assert!(bayer_pattern(0).is_err());
        assert!(bayer_pattern(1).is_err());
        assert!(bayer_pattern(9).is_err());
        assert!(bayer_pattern(encode_filters([0, 1, 0, 2])).is_err());
    }

    #[test]
    fn maps_common_metadata_groups() {
        let color = color();
        let levels = levels(&color).unwrap();
        assert_eq!(levels.black.per_slot, [1, 2, 3, 4]);
        assert_eq!(levels.white, WhiteLevels::Uniform(110));
        assert_eq!(map_orientation(5), Some(8));
        assert_eq!(
            map_default_crop(color.default_crop),
            Some(DefaultCrop {
                x: 1,
                y: 2,
                width: 30,
                height: 40
            })
        );
    }
}
