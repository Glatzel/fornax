use super::bits::V8Bits;
use super::{finish_buffer, pixels, single_payload};
use crate::error::invalid;
use crate::{CodecError, DecodeRequest, PanasonicV8Params, SampleBuffer, SampleFormat};

pub fn decode(
    request: &DecodeRequest<'_>,
    params: &PanasonicV8Params,
) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer
        || !matches!(request.bits_per_sample, 12 | 14 | 16)
    {
        return Err(invalid(
            "Panasonic V8 requires 12-, 14-, or 16-bit integer sample metadata",
        ));
    }
    let sample_count = pixels(request, "Panasonic V8")?;
    single_payload(request, "Panasonic V8")?;
    let strip_area = params.strips.iter().try_fold(0_usize, |total, strip| {
        if strip
            .x
            .checked_add(strip.width)
            .is_none_or(|right| right > request.width)
            || strip
                .y
                .checked_add(strip.height)
                .is_none_or(|bottom| bottom > request.height)
        {
            return Err(invalid("Panasonic V8 strip lies outside the output image"));
        }
        total
            .checked_add(
                strip
                    .width
                    .checked_mul(strip.height)
                    .ok_or_else(|| invalid("Panasonic V8 strip area overflow"))?,
            )
            .ok_or_else(|| invalid("Panasonic V8 strip area total overflow"))
    })?;
    if strip_area != sample_count {
        return Err(invalid(format!(
            "Panasonic V8 strips cover {strip_area} pixels, image requires {sample_count}"
        )));
    }
    for strip in &params.strips {
        strip_bytes(request.source, *strip)?;
    }

    let gamma = gamma_table(params)?;
    let huffman = Huffman::new(&params.huffman_table)?;
    let mut output = vec![0_u16; sample_count];
    for strip in &params.strips {
        decode_strip(
            request.source,
            *strip,
            params,
            &huffman,
            gamma.as_deref(),
            &mut output,
            request.width,
        )?;
    }
    finish_buffer(request, output)
}

fn decode_strip(
    source: &[u8],
    strip: crate::PanasonicV8Strip,
    params: &PanasonicV8Params,
    huffman: &Huffman,
    gamma: Option<&[u16]>,
    output: &mut [u16],
    image_width: usize,
) -> Result<(), CodecError> {
    let bytes = strip_bytes(source, strip)?;
    let data_bits = usize::try_from(strip.data_bits)
        .map_err(|_| invalid("Panasonic V8 strip bit count exceeds usize"))?;
    let mut reader = V8Bits::new(bytes, data_bits)?;
    let mut line = vec![0_u16; strip.width * 2];
    let mut line_base = params.initial_predictors;
    for half_row in 0..strip.height / 2 {
        let mut current = line_base;
        for (group_index, group) in line.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            for index in 0..4 {
                let target_index = match index {
                    0 => 0,
                    1 => 2,
                    2 => 1,
                    3 => 3,
                    _ => unreachable!("V8 groups contain four coefficients"),
                };
                let symbol = huffman.next(&mut reader)?;
                let delta = decode_delta(&mut reader, symbol, &params.shift_down)?;
                let predictor = i32::from(current[target_index]);
                group[target_index] = predictor
                    .checked_add(delta)
                    .ok_or_else(|| invalid("Panasonic V8 predictor overflow"))?
                    .clamp(0, i32::from(params.clip_value))
                    as u16;
            }
            current = [group[0], group[1], group[2], group[3]];
            if group_index == 0 {
                line_base = current;
            }
        }

        let row = strip
            .y
            .checked_add(half_row * 2)
            .ok_or_else(|| invalid("Panasonic V8 output row overflow"))?;
        for column in (0..strip.width).step_by(2) {
            let target = row
                .checked_mul(image_width)
                .and_then(|offset| offset.checked_add(strip.x + column))
                .ok_or_else(|| invalid("Panasonic V8 output offset overflow"))?;
            let next_row = target + image_width;
            output[target] = map(line[column * 2], gamma);
            output[target + 1] = map(line[column * 2 + 1], gamma);
            output[next_row] = map(line[column * 2 + 2], gamma);
            output[next_row + 1] = map(line[column * 2 + 3], gamma);
        }
    }
    Ok(())
}

fn strip_bytes(source: &[u8], strip: crate::PanasonicV8Strip) -> Result<&[u8], CodecError> {
    let byte_count = usize::try_from(strip.data_bits)
        .map_err(|_| invalid("Panasonic V8 strip bit count exceeds usize"))?
        .div_ceil(8);
    let end = strip
        .source_offset
        .checked_add(byte_count)
        .ok_or_else(|| invalid("Panasonic V8 strip range overflow"))?;
    let bytes = source
        .get(strip.source_offset..end)
        .ok_or_else(|| invalid("Panasonic V8 strip payload is out of bounds"))?;
    Ok(bytes)
}

fn decode_delta(
    reader: &mut V8Bits<'_>,
    symbol: usize,
    shift_down: &[u16],
) -> Result<i32, CodecError> {
    let shift = usize::from(shift_down[symbol] & 0x1f);
    let required = symbol.saturating_sub(shift);
    if required == 0 {
        return Ok(if shift == 0 { 0 } else { 1_i32 << (shift - 1) });
    }
    let raw = reader
        .read(u32::try_from(required).map_err(|_| invalid("Panasonic V8 bit count overflow"))?)?
        as i64;
    let sign = raw >> (required - 1);
    let mut value = raw << shift;
    if sign != 1 {
        value -= 1_i64 << symbol;
        if shift == 0 {
            value += 1;
        }
    }
    if shift != 0 {
        value += 1_i64 << (shift - 1);
    }
    i32::try_from(value).map_err(|_| invalid("Panasonic V8 delta exceeds i32"))
}

struct Huffman {
    entries: Vec<(u8, u16, u16)>,
}

impl Huffman {
    fn new(table: &[(u8, u16)]) -> Result<Self, CodecError> {
        let entries = table
            .iter()
            .map(|&(bits, symbol)| {
                if bits == 0 || bits > 16 {
                    return Err(invalid("Panasonic V8 Huffman symbol length is invalid"));
                }
                let mask = u16::MAX << (16 - bits);
                Ok((bits, symbol << (16 - bits), mask))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries })
    }

    fn next(&self, reader: &mut V8Bits<'_>) -> Result<usize, CodecError> {
        let bits = reader.peek_16();
        for (index, &(count, symbol, mask)) in self.entries.iter().enumerate() {
            if bits & mask == symbol {
                reader.skip(usize::from(count))?;
                return Ok(index);
            }
        }
        Err(invalid(
            "Panasonic V8 bitstream has no matching Huffman symbol",
        ))
    }
}

fn gamma_table(params: &PanasonicV8Params) -> Result<Option<Vec<u16>>, CodecError> {
    if params.gamma_points == [65536; 6] && params.gamma_slopes == [0; 6] {
        return Ok(None);
    }
    let mut table = Vec::with_capacity(0x10000);
    for value in 0_u16..=u16::MAX {
        table.push(gamma(value, params)?);
    }
    Ok(Some(table))
}

fn gamma(value: u16, params: &PanasonicV8Params) -> Result<u16, CodecError> {
    let x = u32::from(value);
    let mut region = 0_usize;
    for index in 1..=5 {
        if x >= params.gamma_slopes[index] & 0xffff {
            region = index;
        } else {
            break;
        }
    }
    let point = params.gamma_points[region];
    let slope = params.gamma_slopes[region];
    let mut temporary = x.wrapping_sub(slope & 0xffff);
    let result = if point & 0x1f == 31 {
        if region == 5 {
            0xffff
        } else {
            ((params.gamma_slopes[region + 1] >> 16) & 0xffff) as u16
        }
    } else if point & 0x10 == 0 {
        if point & 0x1f == 15 {
            ((slope >> 16) & 0xffff) as u16
        } else {
            if point & 0x1f != 0 {
                let shift = point & 0x1f;
                temporary = temporary.wrapping_add(1 << (shift - 1)) >> shift;
            }
            temporary.wrapping_add((slope >> 16) & 0xffff) as u16
        }
    } else {
        temporary <<= point & 0xf;
        temporary.wrapping_add((slope >> 16) & 0xffff) as u16
    };
    Ok(result.min(params.clip_value))
}

fn map(value: u16, gamma: Option<&[u16]>) -> u16 {
    gamma.map_or(value, |table| table[usize::from(value)])
}
