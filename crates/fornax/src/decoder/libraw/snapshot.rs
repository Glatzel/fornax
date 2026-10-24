use libraw::{Libraw, UnpackedRawImageKind};

use super::translate_libraw_error;
use crate::error::DecodeError;

/// Adapter-owned copy of the LibRaw fields needed to construct `DecodedRaw`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RawSnapshot {
    pub(super) raw_height: u32,
    pub(super) raw_width: u32,
    pub(super) raw_pitch: u32,
    pub(super) height: u32,
    pub(super) width: u32,
    pub(super) top_margin: u32,
    pub(super) left_margin: u32,
    pub(super) flip: i32,
    pub(super) make: String,
    pub(super) model: String,
    pub(super) normalized_make: String,
    pub(super) normalized_model: String,
    pub(super) cdesc: String,
    pub(super) raw_count: u32,
    pub(super) is_foveon: bool,
    pub(super) dng_version: u32,
    pub(super) colors: i32,
    pub(super) filters: u32,
    pub(super) raw_bps: u32,
    pub(super) black: u32,
    pub(super) cblack: [u32; 6],
    pub(super) maximum: u32,
    pub(super) data_maximum: u32,
    pub(super) linear_max: [u32; 4],
    pub(super) dng_whitelevel: [u32; 4],
    pub(super) default_crop: [u16; 4],
    pub(super) has_color3_image: bool,
    pub(super) has_color4_image: bool,
    pub(super) has_float3_image: bool,
    pub(super) has_float4_image: bool,
    pub(super) progress_flags: u32,
    pub(super) unpack_function: Option<String>,
    pub(super) buffer: Option<RawBuffer>,
}

/// Adapter-owned native sample copy.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum RawBuffer {
    Integer(Vec<u16>),
    Float(Vec<f32>),
}

pub(super) fn snapshot(context: &Libraw) -> Result<RawSnapshot, DecodeError> {
    let sizes = context.get_image_sizes().map_err(translate_libraw_error)?;
    let params = context.get_iparams().map_err(translate_libraw_error)?;
    let color = context.get_color_data().map_err(translate_libraw_error)?;
    let raw = context
        .get_unpacked_raw_image()
        .map_err(translate_libraw_error)?;
    let buffer = match raw.kind() {
        UnpackedRawImageKind::Integer => Some(RawBuffer::Integer(copy_u16_rows(&raw)?)),
        UnpackedRawImageKind::Float => Some(RawBuffer::Float(copy_f32_rows(&raw)?)),
        UnpackedRawImageKind::None => None,
    };

    Ok(RawSnapshot {
        raw_height: u32::from(sizes.raw_height()),
        raw_width: u32::from(sizes.raw_width()),
        raw_pitch: sizes.raw_pitch(),
        height: u32::from(sizes.height()),
        width: u32::from(sizes.width()),
        top_margin: u32::from(sizes.top_margin()),
        left_margin: u32::from(sizes.left_margin()),
        flip: sizes.flip_raw(),
        make: params.make().map_err(translate_libraw_error)?,
        model: params.model().map_err(translate_libraw_error)?,
        normalized_make: params.normalized_make().map_err(translate_libraw_error)?,
        normalized_model: params.normalized_model().map_err(translate_libraw_error)?,
        cdesc: params.cdesc_text().map_err(translate_libraw_error)?,
        raw_count: params.raw_count(),
        is_foveon: params.is_foveon(),
        dng_version: params.dng_version(),
        colors: params.colors(),
        filters: params.filters(),
        raw_bps: color.raw_bps(),
        black: color.black(),
        cblack: color.cblack(),
        maximum: color.maximum(),
        data_maximum: color.data_maximum(),
        linear_max: color.linear_max(),
        dng_whitelevel: color.dng_whitelevel(),
        default_crop: color.dng_default_crop(),
        has_color3_image: raw.has_color3_image(),
        has_color4_image: raw.has_color4_image(),
        has_float3_image: raw.has_float3_image(),
        has_float4_image: raw.has_float4_image(),
        progress_flags: context.progress_flags(),
        unpack_function: context.unpack_function_name(),
        buffer,
    })
}

fn copy_u16_rows(raw: &libraw::UnpackedRawImage<'_>) -> Result<Vec<u16>, DecodeError> {
    let capacity = usize::from(raw.raw_width())
        .checked_mul(usize::from(raw.raw_height()))
        .ok_or_else(allocation_error)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(capacity)
        .map_err(|_| allocation_error())?;
    for row in 0..u32::from(raw.raw_height()) {
        let samples_row = raw
            .integer_row(row)
            .map_err(translate_libraw_error)?
            .ok_or_else(|| DecodeError::Backend("LibRaw raw_image disappeared".into()))?;
        samples.extend_from_slice(samples_row);
    }
    Ok(samples)
}

fn copy_f32_rows(raw: &libraw::UnpackedRawImage<'_>) -> Result<Vec<f32>, DecodeError> {
    let capacity = usize::from(raw.raw_width())
        .checked_mul(usize::from(raw.raw_height()))
        .ok_or_else(allocation_error)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(capacity)
        .map_err(|_| allocation_error())?;
    for row in 0..u32::from(raw.raw_height()) {
        let samples_row = raw
            .float_row(row)
            .map_err(translate_libraw_error)?
            .ok_or_else(|| DecodeError::Backend("LibRaw float_image disappeared".into()))?;
        samples.extend_from_slice(samples_row);
    }
    Ok(samples)
}

fn allocation_error() -> DecodeError {
    DecodeError::Backend("could not allocate the LibRaw sample copy".into())
}
