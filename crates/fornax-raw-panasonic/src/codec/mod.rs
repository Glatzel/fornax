mod bits;
mod packed;
mod v4;
mod v5;
mod v6;
mod v8;

use fornax_codec::ICodecDecode;

use crate::error::invalid;
use crate::samples::{buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, PanasonicV8Params, SampleBuffer};

const MAX_PIXELS: usize = 100_000_000;

#[derive(Clone, Debug)]
pub enum PanasonicCodec {
    V4 { split_blocks: bool },
    V5,
    V6,
    V8 { params: PanasonicV8Params },
    Unpacked12,
    Packed12Control,
}

impl ICodecDecode for PanasonicCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
        match self {
            Self::V4 { split_blocks } => decode_v4(request, *split_blocks),
            Self::V5 => decode_v5(request),
            Self::V6 => decode_v6(request),
            Self::V8 { params } => decode_v8(request, params),
            Self::Unpacked12 => decode_unpacked_12(request),
            Self::Packed12Control => decode_packed_12_control(request),
        }
    }
}

pub fn decode_v4(
    request: &DecodeRequest<'_>,
    split_blocks: bool,
) -> Result<SampleBuffer, CodecError> {
    v4::decode(request, split_blocks)
}

pub fn decode_v5(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    v5::decode(request)
}

pub fn decode_v6(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    v6::decode(request)
}

pub fn decode_v8(
    request: &DecodeRequest<'_>,
    params: &PanasonicV8Params,
) -> Result<SampleBuffer, CodecError> {
    v8::decode(request, params)
}

pub fn decode_unpacked_12(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    packed::decode_unpacked_12(request)
}

pub fn decode_packed_12_control(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    packed::decode_packed_12_control(request)
}

fn pixels(request: &DecodeRequest<'_>, format: &str) -> Result<usize, CodecError> {
    if request.width == 0 || request.height == 0 {
        return Err(invalid(format!("{format} dimensions must not be zero")));
    }
    let pixels = request
        .width
        .checked_mul(request.height)
        .ok_or_else(|| invalid(format!("{format} sample count overflow")))?;
    if pixels > MAX_PIXELS {
        return Err(invalid(format!(
            "{format} dimensions {0}x{1} exceed the {MAX_PIXELS}-pixel limit",
            request.width, request.height
        )));
    }
    Ok(pixels)
}

fn single_payload<'a>(
    request: &'a DecodeRequest<'_>,
    format: &str,
) -> Result<&'a [u8], CodecError> {
    let [strip] = request.chunks else {
        return Err(invalid(format!(
            "{format} requires exactly one payload chunk"
        )));
    };
    chunk(request.source, *strip)
}

fn finish_buffer(
    request: &DecodeRequest<'_>,
    samples: Vec<u16>,
) -> Result<SampleBuffer, CodecError> {
    buffer_u16(request.width, request.height, samples)
}

fn require_integer_12_or_14(request: &DecodeRequest<'_>, format: &str) -> Result<u16, CodecError> {
    if request.sample_format != crate::SampleFormat::Integer {
        return Err(invalid(format!("{format} requires integer samples")));
    }
    match request.bits_per_sample {
        bits @ (12 | 14) => Ok(bits),
        bits => Err(invalid(format!(
            "{format} does not support {bits}-bit samples"
        ))),
    }
}
