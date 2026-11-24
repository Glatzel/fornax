//! High-level TIFF image decoding on [`ParsedTiff`].

use fornax_codec::{ByteOrder, DecodeRequest, ICodecDecode, Predictor, SampleBuffer, SampleFormat};
use fornax_core::{MonoImage, RgbImage, RgbaImage, UvImage};

use crate::constants::{PhotometricInterpretation, PlanarConfiguration, SampleFormatTag, TiffTag};
use crate::image::{
    TiffImage, TiffIoError, TypedTiffImage, deinterleave_planes, interleave_planes,
};
use crate::raster::{RasterError, RasterLayout, resolve_tiff_compression};
use crate::value::{unsigned_scalar, unsigned_vec_lookup};
use crate::{Directory, ParsedTiff, Tag};

impl ParsedTiff {
    /// Decodes directory `directory_index` into a [`TiffImage`].
    ///
    /// Channel count mapping:
    /// - `1` -> [`TypedTiffImage::Mono`]
    /// - `2` -> [`TypedTiffImage::Uv`]
    /// - `3` -> [`TypedTiffImage::Rgb`]
    /// - `4` -> [`TypedTiffImage::Rgba`]
    /// - `>= 5` -> [`TypedTiffImage::MultiPlane`]
    pub fn decode_image(&self, directory_index: usize) -> Result<TiffImage, TiffIoError> {
        read_directory_internal(self, directory_index, false)
    }

    /// Decodes directory `directory_index` as [`TypedTiffImage::MultiPlane`]
    /// regardless of channel count.
    pub fn decode_planes(&self, directory_index: usize) -> Result<TiffImage, TiffIoError> {
        read_directory_internal(self, directory_index, true)
    }
}

fn read_directory_internal(
    parsed: &ParsedTiff,
    directory_index: usize,
    force_multiplane: bool,
) -> Result<TiffImage, TiffIoError> {
    let directory = parsed.directories.get(directory_index).ok_or_else(|| {
        TiffIoError::Invalid(format!("TIFF directory {directory_index} does not exist").into())
    })?;

    let samples_per_pixel = unsigned_scalar(directory.get(&TiffTag::SamplesPerPixel.into()))
        .map(|value| {
            usize::try_from(value)
                .map_err(|_| TiffIoError::Invalid("SamplesPerPixel exceeds usize".into()))
        })
        .transpose()?
        .unwrap_or(1);
    if samples_per_pixel == 0 {
        return Err(TiffIoError::Invalid(
            "SamplesPerPixel must not be zero".into(),
        ));
    }

    let planar_raw = unsigned_scalar(directory.get(&TiffTag::PlanarConfiguration.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("PlanarConfiguration exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let planar = match PlanarConfiguration::from(planar_raw) {
        PlanarConfiguration::Unknown(id) => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported PlanarConfiguration {id}").into(),
            ));
        }
        other => other,
    };

    let photometric_raw =
        unsigned_scalar(directory.get(&TiffTag::PhotometricInterpretation.into())).map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("PhotometricInterpretation exceeds u16".into()))
        });
    let photometric = match photometric_raw {
        Some(Ok(id)) => PhotometricInterpretation::from(id),
        Some(Err(err)) => return Err(err),
        None => {
            if samples_per_pixel <= 2 {
                PhotometricInterpretation::BlackIsZero
            } else {
                PhotometricInterpretation::Rgb
            }
        }
    };
    match photometric {
        PhotometricInterpretation::WhiteIsZero
        | PhotometricInterpretation::BlackIsZero
        | PhotometricInterpretation::Rgb => {}
        other => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported PhotometricInterpretation {other:?}").into(),
            ));
        }
    }

    let bits_per_sample = parse_uniform_u16_tag(directory, TiffTag::BitsPerSample, 1)?;
    let sample_format_raw = parse_uniform_u16_tag(directory, TiffTag::SampleFormat, 1)?;
    let sample_format = match SampleFormatTag::from(sample_format_raw) {
        SampleFormatTag::Unsigned => SampleFormat::Integer,
        SampleFormatTag::Float => SampleFormat::Float,
        other => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported SampleFormat {other:?}").into(),
            ));
        }
    };

    let compression_id = unsigned_scalar(directory.get(&TiffTag::Compression.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("Compression tag exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let compression = resolve_tiff_compression(compression_id)?;

    let predictor_id = unsigned_scalar(directory.get(&TiffTag::Predictor.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("Predictor tag exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let predictor = resolve_predictor(predictor_id, planar, samples_per_pixel)?;

    let layout = RasterLayout::from_lookup_with_planes(directory, samples_per_pixel, planar)?;
    let source = parsed.source.as_ref();
    validate_chunk_bounds(&layout, source.len())?;
    let byte_order = match parsed.byte_order() {
        crate::ByteOrder::Little => ByteOrder::Little,
        crate::ByteOrder::Big => ByteOrder::Big,
    };

    let invert_white_is_zero = photometric == PhotometricInterpretation::WhiteIsZero
        && sample_format == SampleFormat::Integer;

    if planar == PlanarConfiguration::Planar || samples_per_pixel == 1 {
        let mut buffers = Vec::with_capacity(samples_per_pixel);
        for plane in 0..samples_per_pixel {
            let chunks = layout
                .plane_chunks(plane)
                .ok_or_else(|| TiffIoError::Invalid("missing planar chunk slice".into()))?;
            let request = DecodeRequest {
                source,
                width: layout.width(),
                height: layout.height(),
                chunk_width: layout.chunk_width(),
                chunk_height: layout.chunk_height(),
                chunks,
                bits_per_sample,
                sample_format,
                byte_order,
                predictor,
            };
            buffers.push(compression.decode(&request)?);
        }
        buffers_to_tiff_image(
            buffers,
            force_multiplane,
            invert_white_is_zero,
            bits_per_sample,
        )
    } else {
        let interleaved_width = layout
            .width()
            .checked_mul(samples_per_pixel)
            .ok_or_else(|| TiffIoError::Invalid("interleaved image width overflow".into()))?;
        let interleaved_chunk_width = layout
            .chunk_width()
            .checked_mul(samples_per_pixel)
            .ok_or_else(|| TiffIoError::Invalid("interleaved chunk width overflow".into()))?;
        let request = DecodeRequest {
            source,
            width: interleaved_width,
            height: layout.height(),
            chunk_width: interleaved_chunk_width,
            chunk_height: layout.chunk_height(),
            chunks: layout.chunks(),
            bits_per_sample,
            sample_format,
            byte_order,
            predictor,
        };
        let buffer = compression.decode(&request)?;
        chunky_buffer_to_tiff_image(
            buffer,
            layout.width(),
            layout.height(),
            samples_per_pixel,
            force_multiplane,
            invert_white_is_zero,
            bits_per_sample,
        )
    }
}

fn parse_uniform_u16_tag(
    directory: &Directory,
    tag: TiffTag,
    default: u16,
) -> Result<u16, TiffIoError> {
    let tiff_tag = Tag::from(tag);
    if !directory.contains_key(&tiff_tag) {
        return Ok(default);
    }
    let values = unsigned_vec_lookup::<_, _, u16>(directory, tiff_tag)
        .map_err(|_| TiffIoError::Invalid(format!("{tag} has an invalid unsigned value").into()))?;
    let first = *values
        .first()
        .ok_or_else(|| TiffIoError::Invalid(format!("{tag} is empty").into()))?;
    if values.iter().any(|&value| value != first) {
        return Err(TiffIoError::Unsupported(
            format!("non-uniform {tag} across channels is unsupported").into(),
        ));
    }
    Ok(first)
}

fn resolve_predictor(
    id: u16,
    planar: PlanarConfiguration,
    samples_per_pixel: usize,
) -> Result<Predictor, TiffIoError> {
    match id {
        1 => Ok(Predictor::None),
        2 => {
            if planar == PlanarConfiguration::Planar || samples_per_pixel == 1 {
                Ok(Predictor::HorizontalInteger)
            } else {
                Ok(Predictor::HorizontalIntegerStrided { samples_per_pixel })
            }
        }
        3 => {
            let byte_stride = if planar == PlanarConfiguration::Planar {
                1
            } else {
                samples_per_pixel
            };
            Ok(Predictor::FloatingPoint { byte_stride })
        }
        _ => Err(TiffIoError::Unsupported(
            format!("unsupported TIFF predictor {id}").into(),
        )),
    }
}

fn validate_chunk_bounds(layout: &RasterLayout, source_len: usize) -> Result<(), TiffIoError> {
    for (index, chunk) in layout.chunks().iter().copied().enumerate() {
        let end = chunk
            .offset
            .checked_add(chunk.byte_count)
            .ok_or(RasterError::ChunkRangeOverflow { index })?;
        if end > source_len {
            return Err(RasterError::ChunkOutOfBounds {
                index,
                offset: chunk.offset,
                byte_count: chunk.byte_count,
                source_len,
            }
            .into());
        }
    }
    Ok(())
}

fn invert_white_is_zero_u8(data: &mut [u8], bits: u16) {
    let max_val = ((1_u16 << bits) - 1) as u8;
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn invert_white_is_zero_u16(data: &mut [u16], bits: u16) {
    let max_val = if bits == 16 {
        u16::MAX
    } else {
        (1_u16 << bits) - 1
    };
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn invert_white_is_zero_u32(data: &mut [u32], bits: u16) {
    let max_val = if bits == 32 {
        u32::MAX
    } else {
        (1_u32 << bits) - 1
    };
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn chunky_buffer_to_tiff_image(
    buffer: SampleBuffer,
    width: usize,
    height: usize,
    samples_per_pixel: usize,
    force_multiplane: bool,
    invert_white: bool,
    bits: u16,
) -> Result<TiffImage, TiffIoError> {
    let w = u32::try_from(width).map_err(|_| TiffIoError::Invalid("width exceeds u32".into()))?;
    let h = u32::try_from(height).map_err(|_| TiffIoError::Invalid("height exceeds u32".into()))?;

    macro_rules! convert {
        ($samples:expr, $variant:ident, $invert:expr) => {{
            let mut raw = $samples;
            if invert_white {
                $invert(&mut raw, bits);
            }
            if force_multiplane || samples_per_pixel >= 5 {
                let planes = deinterleave_planes(w, h, samples_per_pixel, &raw)?;
                Ok(TiffImage::$variant(TypedTiffImage::MultiPlane(planes)))
            } else {
                Ok(TiffImage::$variant(match samples_per_pixel {
                    1 => TypedTiffImage::Mono(
                        MonoImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid mono buffer".into()))?,
                    ),
                    2 => TypedTiffImage::Uv(
                        UvImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid UV buffer".into()))?,
                    ),
                    3 => TypedTiffImage::Rgb(
                        RgbImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGB buffer".into()))?,
                    ),
                    4 => TypedTiffImage::Rgba(
                        RgbaImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGBA buffer".into()))?,
                    ),
                    _ => unreachable!(),
                }))
            }
        }};
    }

    match buffer {
        SampleBuffer::U8(buf) => convert!(buf.into_raw(), U8, invert_white_is_zero_u8),
        SampleBuffer::U16(buf) => convert!(buf.into_raw(), U16, invert_white_is_zero_u16),
        SampleBuffer::U32(buf) => convert!(buf.into_raw(), U32, invert_white_is_zero_u32),
        SampleBuffer::F32(buf) => convert!(buf.into_raw(), F32, |_: &mut [f32], _| {}),
        SampleBuffer::F64(buf) => convert!(buf.into_raw(), F64, |_: &mut [f64], _| {}),
        _ => Err(TiffIoError::Unsupported(
            "unsupported decoded sample buffer variant".into(),
        )),
    }
}

fn buffers_to_tiff_image(
    buffers: Vec<SampleBuffer>,
    force_multiplane: bool,
    invert_white: bool,
    bits: u16,
) -> Result<TiffImage, TiffIoError> {
    let first = buffers
        .first()
        .ok_or_else(|| TiffIoError::Invalid("decoded image has no planes".into()))?;
    let w = first.width();
    let h = first.height();

    macro_rules! assemble {
        ($variant:ident, $into_raw:ident, $invert:expr) => {{
            let mut planes = Vec::with_capacity(buffers.len());
            for (index, buf) in buffers.into_iter().enumerate() {
                let mut raw = buf.$into_raw().ok_or_else(|| {
                    TiffIoError::Invalid("mixed sample types across planes".into())
                })?;
                if invert_white && index == 0 {
                    $invert(&mut raw, bits);
                }
                let plane = MonoImage::from_raw(w, h, raw)
                    .ok_or_else(|| TiffIoError::Invalid("invalid plane dimensions".into()))?;
                planes.push(plane);
            }
            if force_multiplane || planes.len() >= 5 {
                Ok(TiffImage::$variant(TypedTiffImage::MultiPlane(planes)))
            } else {
                Ok(TiffImage::$variant(match planes.len() {
                    1 => TypedTiffImage::Mono(planes.into_iter().next().unwrap()),
                    2 => TypedTiffImage::Uv(
                        UvImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid UV buffer".into()))?,
                    ),
                    3 => TypedTiffImage::Rgb(
                        RgbImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGB buffer".into()))?,
                    ),
                    4 => TypedTiffImage::Rgba(
                        RgbaImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGBA buffer".into()))?,
                    ),
                    _ => unreachable!(),
                }))
            }
        }};
    }

    match first {
        SampleBuffer::U8(_) => assemble!(U8, into_raw_u8, invert_white_is_zero_u8),
        SampleBuffer::U16(_) => assemble!(U16, into_raw_u16, invert_white_is_zero_u16),
        SampleBuffer::U32(_) => assemble!(U32, into_raw_u32, invert_white_is_zero_u32),
        SampleBuffer::F32(_) => assemble!(F32, into_raw_f32, |_: &mut [f32], _| {}),
        SampleBuffer::F64(_) => assemble!(F64, into_raw_f64, |_: &mut [f64], _| {}),
        _ => Err(TiffIoError::Unsupported(
            "unsupported decoded sample buffer variant".into(),
        )),
    }
}
