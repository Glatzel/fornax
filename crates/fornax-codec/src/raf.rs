use crate::error::{invalid, unsupported};
use crate::{ByteOrder, CodecError, DecodeRequest, FujiCfa, SampleBuffer};

const BLOCK_WIDTH: usize = 0x300;
const BLOCK_ROWS: usize = 6;
const LINE_BUFFERS: usize = 18;
const R: usize = 0;
const G: usize = 1;
const B: usize = 2;
const R0: usize = 0;
const R1: usize = 1;
const R2: usize = 2;
const R3: usize = 3;
const R4: usize = 4;
const G0: usize = 5;
const G1: usize = 6;
const G2: usize = 7;
const G3: usize = 8;
const G4: usize = 9;
const G5: usize = 10;
const G6: usize = 11;
const G7: usize = 12;
const B0: usize = 13;
const B1: usize = 14;
const B2: usize = 15;
const B3: usize = 16;
const B4: usize = 17;

pub fn decode_packed(
    request: &DecodeRequest<'_>,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    if bits != 12 {
        return Err(unsupported(format!(
            "Fujifilm packed samples at {bits} bits are unsupported"
        )));
    }
    if byte_order != ByteOrder::Little {
        return Err(unsupported(
            "Fujifilm packed 12-bit samples are little-endian",
        ));
    }
    let payload = single_payload(request)?;
    let pixels = checked_pixels(request.width, request.height)?;
    let bytes = pixels
        .checked_mul(3)
        .ok_or_else(|| invalid("Fujifilm packed byte count overflow"))?
        .div_ceil(2);
    if payload.len() < bytes {
        return Err(invalid("Fujifilm packed payload is truncated"));
    }
    let mut samples = Vec::with_capacity(pixels);
    for pair in payload[..bytes].as_chunks::<3>().0 {
        samples.push(u16::from(pair[0]) | (u16::from(pair[1] & 0x0f) << 8));
        samples.push((u16::from(pair[1]) >> 4) | (u16::from(pair[2]) << 4));
    }
    samples.truncate(pixels);
    SampleBuffer::from_u16(
        u32::try_from(request.width).map_err(|_| invalid("Fujifilm width exceeds u32"))?,
        u32::try_from(request.height).map_err(|_| invalid("Fujifilm height exceeds u32"))?,
        samples,
    )
    .map_err(|error| invalid(error.to_string()))
}

pub fn decode_lossless(
    request: &DecodeRequest<'_>,
    cfa: FujiCfa,
) -> Result<SampleBuffer, CodecError> {
    let payload = single_payload(request)?;
    let header = parse_header(payload)?;
    let expected_kind = match cfa {
        FujiCfa::Bayer(_) => 0,
        FujiCfa::XTrans(_) => 16,
    };
    if header.raw_kind != expected_kind {
        return Err(invalid(
            "Fujifilm lossless raw type does not match the CFA dimensions",
        ));
    }
    if usize::from(header.width) != request.width || usize::from(header.height) != request.height {
        return Err(invalid(format!(
            "Fujifilm lossless header dimensions {}x{} do not match request dimensions {}x{}",
            header.width, header.height, request.width, request.height
        )));
    }
    let params = Params::new(header)?;
    let block_sizes = parse_block_sizes(payload, header)?;
    let pixels = checked_pixels(request.width, request.height)?;
    let mut output = vec![0_u16; pixels];
    let mut block_offset = block_table_offset(header);
    for (block_index, block_size) in block_sizes.into_iter().enumerate() {
        let block_end = block_offset
            .checked_add(block_size)
            .ok_or_else(|| invalid("Fujifilm block range overflow"))?;
        let block = payload
            .get(block_offset..block_end)
            .ok_or_else(|| invalid("Fujifilm block is truncated"))?;
        let mut reader = BitReader::new(block);
        let block_x = block_index
            .checked_mul(BLOCK_WIDTH)
            .ok_or_else(|| invalid("Fujifilm block x offset overflow"))?;
        let block_width = BLOCK_WIDTH.min(request.width - block_x);
        let geometry = BlockGeometry {
            cfa,
            block_index,
            block_width,
            image_width: request.width,
        };
        decode_block(&mut reader, &params, header, &geometry, &mut output)?;
        block_offset = block_end;
    }
    let trailing = payload.len().saturating_sub(block_offset);
    if trailing > 16 {
        return Err(invalid(format!(
            "Fujifilm lossless payload has {trailing} unexpected trailing bytes"
        )));
    }
    SampleBuffer::from_u16(
        u32::try_from(request.width).map_err(|_| invalid("Fujifilm width exceeds u32"))?,
        u32::try_from(request.height).map_err(|_| invalid("Fujifilm height exceeds u32"))?,
        output,
    )
    .map_err(|error| invalid(error.to_string()))
}

fn single_payload<'a>(request: &'a DecodeRequest<'_>) -> Result<&'a [u8], CodecError> {
    let [chunk] = request.chunks else {
        return Err(invalid(
            "Fujifilm image must contain exactly one payload chunk",
        ));
    };
    let end = chunk
        .offset
        .checked_add(chunk.byte_count)
        .ok_or_else(|| invalid("Fujifilm payload range overflow"))?;
    request
        .source
        .get(chunk.offset..end)
        .ok_or_else(|| invalid("Fujifilm payload is out of bounds"))
}

fn checked_pixels(width: usize, height: usize) -> Result<usize, CodecError> {
    if width == 0 || height == 0 {
        return Err(invalid("Fujifilm dimensions must not be zero"));
    }
    width
        .checked_mul(height)
        .ok_or_else(|| invalid("Fujifilm sample count overflow"))
}

#[derive(Clone, Copy, Debug)]
struct Header {
    lossless: bool,
    raw_kind: u8,
    bits: u8,
    height: u16,
    rounded_width: u16,
    width: u16,
    block_width: u16,
    blocks: u8,
    block_lines: u16,
}

fn parse_header(payload: &[u8]) -> Result<Header, CodecError> {
    if payload.len() < 16 {
        return Err(invalid("Fujifilm lossless header is truncated"));
    }
    let mut reader = BigEndianReader::new(payload);
    let marker = reader.u16()?;
    let lossless = reader.u8()?;
    let raw_kind = reader.u8()?;
    let bits = reader.u8()?;
    let height = reader.u16()?;
    let rounded_width = reader.u16()?;
    let width = reader.u16()?;
    let block_width = reader.u16()?;
    let blocks = reader.u8()?;
    let block_lines = reader.u16()?;
    let header = Header {
        lossless: lossless == 1,
        raw_kind,
        bits,
        height,
        rounded_width,
        width,
        block_width,
        blocks,
        block_lines,
    };
    if marker != 0x4953 {
        return Err(invalid("Fujifilm lossless header signature is invalid"));
    }
    if !header.lossless {
        return Err(unsupported("Fujifilm lossy RAF compression"));
    }
    if !matches!(header.bits, 12 | 14 | 16) {
        return Err(invalid(format!(
            "Fujifilm lossless bit depth {} is invalid",
            header.bits
        )));
    }
    if !matches!(header.raw_kind, 0 | 16) {
        return Err(invalid(format!(
            "Fujifilm raw type {} is invalid",
            header.raw_kind
        )));
    }
    if header.height == 0
        || header.height > 0x3000
        || !header.height.is_multiple_of(6)
        || header.width < 0x300
        || header.width > 0x3000
        || !header.width.is_multiple_of(24)
        || header.rounded_width > 0x3000
        || header.block_width != 0x300
        || !header.rounded_width.is_multiple_of(header.block_width)
        || header.blocks == 0
        || header.blocks > 16
        || u16::from(header.blocks) != header.rounded_width / header.block_width
        || header.rounded_width < header.width
        || header.rounded_width - header.width >= header.block_width
        || header.block_lines == 0
        || header.block_lines > 0x800
        || header.block_lines != header.height / 6
    {
        return Err(invalid("Fujifilm lossless header geometry is invalid"));
    }
    Ok(header)
}

fn parse_block_sizes(payload: &[u8], header: Header) -> Result<Vec<usize>, CodecError> {
    let mut reader = BigEndianReader::new(payload);
    reader.position = 16;
    let mut sizes = Vec::with_capacity(usize::from(header.blocks));
    let mut total = 0_usize;
    for _ in 0..header.blocks {
        let size = usize::try_from(reader.u32()?)
            .map_err(|_| invalid("Fujifilm block size exceeds the address space"))?;
        if size == 0 {
            return Err(invalid("Fujifilm block size must not be zero"));
        }
        total = total
            .checked_add(size)
            .ok_or_else(|| invalid("Fujifilm block size total overflow"))?;
        sizes.push(size);
    }
    let data_start = block_table_offset(header);
    let available = payload.len().saturating_sub(data_start);
    if total > available {
        return Err(invalid(format!(
            "Fujifilm block table requires {total} bytes, found {available}"
        )));
    }
    Ok(sizes)
}

const fn block_table_offset(header: Header) -> usize {
    16 + (header.blocks as usize * 4).next_multiple_of(16)
}

#[derive(Clone, Debug)]
struct Params {
    q_table: Vec<i32>,
    total_values: i32,
    max_bits: u32,
    max_value: i32,
    line_width: usize,
    min_value: i32,
}

impl Params {
    fn new(header: Header) -> Result<Self, CodecError> {
        let max_value = (1_i32 << header.bits) - 1;
        let q_points = [0, 0x12, 0x43, 0x114, max_value];
        let total_values = max_value + 1;
        let q_table = build_q_table(header.bits, &q_points)?;
        let line_width = if header.raw_kind == 16 {
            BLOCK_WIDTH * 2 / 3
        } else {
            BLOCK_WIDTH / 2
        };
        Ok(Self {
            q_table,
            total_values,
            max_bits: u32::from(header.bits) * 4,
            max_value,
            line_width,
            min_value: 0x40,
        })
    }

    fn gradient(&self, first: i32, second: i32) -> Result<(usize, i8), CodecError> {
        let index1 = self
            .max_value
            .checked_add(first)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < self.q_table.len())
            .ok_or_else(|| invalid("Fujifilm gradient index is out of bounds"))?;
        let index2 = self
            .max_value
            .checked_add(second)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < self.q_table.len())
            .ok_or_else(|| invalid("Fujifilm gradient index is out of bounds"))?;
        let value = 9 * self.q_table[index1] + self.q_table[index2];
        if !(-40..=40).contains(&value) {
            return Err(invalid("Fujifilm gradient classification is invalid"));
        }
        Ok((value.unsigned_abs() as usize, value as i8))
    }
}

fn build_q_table(bits: u8, points: &[i32; 5]) -> Result<Vec<i32>, CodecError> {
    let length = 2_usize
        .checked_shl(u32::from(bits))
        .ok_or_else(|| invalid("Fujifilm quantization table size overflow"))?;
    let mut table = vec![0_i32; length];
    for (current, value) in (-points[4]..).zip(&mut table) {
        if current > points[4] {
            break;
        }
        *value = if current <= -points[3] {
            -4
        } else if current <= -points[2] {
            -3
        } else if current <= -points[1] {
            -2
        } else if current < -points[0] {
            -1
        } else if current <= points[0] {
            0
        } else if current < points[1] {
            1
        } else if current < points[2] {
            2
        } else if current < points[3] {
            3
        } else {
            4
        };
    }
    Ok(table)
}

#[derive(Clone, Copy, Debug)]
struct Gradient {
    divisor: i32,
    shift: i32,
}

#[derive(Clone, Debug)]
struct Block {
    gradients_even: [[Gradient; 41]; 3],
    gradients_odd: [[Gradient; 41]; 3],
    lines: [Vec<u16>; LINE_BUFFERS],
}

impl Block {
    fn new(params: &Params) -> Self {
        let initial = Gradient {
            divisor: 2.max((params.total_values + 0x20) >> 6),
            shift: 1,
        };
        Self {
            gradients_even: [[initial; 41]; 3],
            gradients_odd: [[initial; 41]; 3],
            lines: std::array::from_fn(|_| vec![0; params.line_width + 2]),
        }
    }

    fn extend(&mut self, first: usize, last: usize) {
        for line in first..=last {
            self.lines[line][0] = self.lines[line - 1][1];
            let width = self.lines[line].len() - 1;
            self.lines[line][width] = self.lines[line - 1][width - 1];
        }
    }
}

#[derive(Clone, Copy)]
struct ColorPosition {
    even: usize,
    odd: usize,
}

impl ColorPosition {
    const fn new() -> Self { Self { even: 0, odd: 1 } }
    const fn reset(&mut self) { *self = Self::new(); }
}

fn decode_block(
    reader: &mut BitReader<'_>,
    params: &Params,
    header: Header,
    geometry: &BlockGeometry,
    output: &mut [u16],
) -> Result<(), CodecError> {
    let mut block = Block::new(params);
    for block_line in 0..usize::from(header.block_lines) {
        if header.raw_kind == 16 {
            decode_xtrans_lines(&mut block, reader, params)?;
        } else {
            decode_bayer_lines(&mut block, reader, params)?;
        }
        shift_previous_lines(&mut block);
        copy_lines(&block, geometry, block_line, output);
        clear_current_lines(&mut block, params.line_width);
    }
    Ok(())
}

fn decode_bayer_lines(
    block: &mut Block,
    reader: &mut BitReader<'_>,
    params: &Params,
) -> Result<(), CodecError> {
    let width = params.line_width;
    let mut colors = [ColorPosition::new(); 3];
    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                R2,
                &mut colors[R].even,
                &mut block.gradients_even[0],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                G2,
                &mut colors[G].even,
                &mut block.gradients_even[0],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R2,
                &mut colors[R].odd,
                &mut block.gradients_odd[0],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G2,
                &mut colors[G].odd,
                &mut block.gradients_odd[0],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                G3,
                &mut colors[G].even,
                &mut block.gradients_even[1],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                B2,
                &mut colors[B].even,
                &mut block.gradients_even[1],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G3,
                &mut colors[G].odd,
                &mut block.gradients_odd[1],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B2,
                &mut colors[B].odd,
                &mut block.gradients_odd[1],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    colors[R].reset();
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                R3,
                &mut colors[R].even,
                &mut block.gradients_even[2],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                G4,
                &mut colors[G].even,
                &mut block.gradients_even[2],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R3,
                &mut colors[R].odd,
                &mut block.gradients_odd[2],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G4,
                &mut colors[G].odd,
                &mut block.gradients_odd[2],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();
    colors[B].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                G5,
                &mut colors[G].even,
                &mut block.gradients_even[0],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                B3,
                &mut colors[B].even,
                &mut block.gradients_even[0],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G5,
                &mut colors[G].odd,
                &mut block.gradients_odd[0],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B3,
                &mut colors[B].odd,
                &mut block.gradients_odd[0],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    colors[R].reset();
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                R4,
                &mut colors[R].even,
                &mut block.gradients_even[1],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                G6,
                &mut colors[G].even,
                &mut block.gradients_even[1],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R4,
                &mut colors[R].odd,
                &mut block.gradients_odd[1],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G6,
                &mut colors[G].odd,
                &mut block.gradients_odd[1],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();
    colors[B].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                G7,
                &mut colors[G].even,
                &mut block.gradients_even[2],
            )?;
            decode_even(
                reader,
                params,
                &mut block.lines,
                B4,
                &mut colors[B].even,
                &mut block.gradients_even[2],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G7,
                &mut colors[G].odd,
                &mut block.gradients_odd[2],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B4,
                &mut colors[B].odd,
                &mut block.gradients_odd[2],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    Ok(())
}

fn decode_xtrans_lines(
    block: &mut Block,
    reader: &mut BitReader<'_>,
    params: &Params,
) -> Result<(), CodecError> {
    let width = params.line_width;
    let mut colors = [ColorPosition::new(); 3];

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            interpolate_even(&mut block.lines, R2, &mut colors[R].even);
            decode_even(
                reader,
                params,
                &mut block.lines,
                G2,
                &mut colors[G].even,
                &mut block.gradients_even[0],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R2,
                &mut colors[R].odd,
                &mut block.gradients_odd[0],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G2,
                &mut colors[G].odd,
                &mut block.gradients_odd[0],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                G3,
                &mut colors[G].even,
                &mut block.gradients_even[1],
            )?;
            interpolate_even(&mut block.lines, B2, &mut colors[B].even);
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G3,
                &mut colors[G].odd,
                &mut block.gradients_odd[1],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B2,
                &mut colors[B].odd,
                &mut block.gradients_odd[1],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    colors[R].reset();
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            if colors[R].even & 3 != 0 {
                decode_even(
                    reader,
                    params,
                    &mut block.lines,
                    R3,
                    &mut colors[R].even,
                    &mut block.gradients_even[2],
                )?;
            } else {
                interpolate_even(&mut block.lines, R3, &mut colors[R].even);
            }
            interpolate_even(&mut block.lines, G4, &mut colors[G].even);
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R3,
                &mut colors[R].odd,
                &mut block.gradients_odd[2],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G4,
                &mut colors[G].odd,
                &mut block.gradients_odd[2],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();
    colors[B].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            decode_even(
                reader,
                params,
                &mut block.lines,
                G5,
                &mut colors[G].even,
                &mut block.gradients_even[0],
            )?;
            if colors[B].even & 3 == 2 {
                interpolate_even(&mut block.lines, B3, &mut colors[B].even);
            } else {
                decode_even(
                    reader,
                    params,
                    &mut block.lines,
                    B3,
                    &mut colors[B].even,
                    &mut block.gradients_even[0],
                )?;
            }
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G5,
                &mut colors[G].odd,
                &mut block.gradients_odd[0],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B3,
                &mut colors[B].odd,
                &mut block.gradients_odd[0],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    colors[R].reset();
    colors[G].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            if colors[R].even & 3 == 2 {
                interpolate_even(&mut block.lines, R4, &mut colors[R].even);
            } else {
                decode_even(
                    reader,
                    params,
                    &mut block.lines,
                    R4,
                    &mut colors[R].even,
                    &mut block.gradients_even[1],
                )?;
            }
            decode_even(
                reader,
                params,
                &mut block.lines,
                G6,
                &mut colors[G].even,
                &mut block.gradients_even[1],
            )?;
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                R4,
                &mut colors[R].odd,
                &mut block.gradients_odd[1],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G6,
                &mut colors[G].odd,
                &mut block.gradients_odd[1],
            )?;
        }
    }
    block.extend(R2, R4);
    block.extend(G2, G7);
    colors[G].reset();
    colors[B].reset();

    while colors[G].even < width || colors[G].odd < width {
        if colors[G].even < width {
            interpolate_even(&mut block.lines, G7, &mut colors[G].even);
            if colors[B].even & 3 != 0 {
                decode_even(
                    reader,
                    params,
                    &mut block.lines,
                    B4,
                    &mut colors[B].even,
                    &mut block.gradients_even[2],
                )?;
            } else {
                interpolate_even(&mut block.lines, B4, &mut colors[B].even);
            }
        }
        if colors[G].even > 8 {
            decode_odd(
                reader,
                params,
                &mut block.lines,
                G7,
                &mut colors[G].odd,
                &mut block.gradients_odd[2],
            )?;
            decode_odd(
                reader,
                params,
                &mut block.lines,
                B4,
                &mut colors[B].odd,
                &mut block.gradients_odd[2],
            )?;
        }
    }
    block.extend(G2, G7);
    block.extend(B2, B4);
    Ok(())
}

fn decode_even(
    reader: &mut BitReader<'_>,
    params: &Params,
    lines: &mut [Vec<u16>],
    line: usize,
    position: &mut usize,
    gradients: &mut [Gradient; 41],
) -> Result<(), CodecError> {
    let above = lines[line - 1][1 + *position] as i32;
    let left = lines[line - 1][*position] as i32;
    let right = lines[line - 1][2 + *position] as i32;
    let upper = lines[line - 2][1 + *position] as i32;
    let diff_left = (left - above).abs();
    let diff_upper = (upper - above).abs();
    let diff_right = (right - above).abs();
    let (gradient_index, gradient_sign) = params.gradient(above - upper, left - above)?;
    let mut prediction = if diff_left > diff_upper && diff_left > diff_right {
        upper + right + 2 * above
    } else if diff_right > diff_left && diff_right > diff_upper {
        upper + left + 2 * above
    } else {
        right + left + 2 * above
    };
    let code = read_code(reader, params, &mut gradients[gradient_index])?;
    prediction >>= 2;
    if gradient_sign < 0 {
        prediction -= code;
    } else {
        prediction += code;
    }
    prediction = wrap_prediction(prediction, params);
    lines[line][1 + *position] = clamp_sample(prediction, params.max_value);
    *position += 2;
    Ok(())
}

fn decode_odd(
    reader: &mut BitReader<'_>,
    params: &Params,
    lines: &mut [Vec<u16>],
    line: usize,
    position: &mut usize,
    gradients: &mut [Gradient; 41],
) -> Result<(), CodecError> {
    let left = lines[line][*position] as i32;
    let above = lines[line - 1][1 + *position] as i32;
    let upper_left = lines[line - 1][*position] as i32;
    let upper_right = lines[line - 1][2 + *position] as i32;
    let right = lines[line][2 + *position] as i32;
    let (gradient_index, gradient_sign) = params.gradient(above - upper_left, upper_left - left)?;
    let mut prediction = if (above > upper_left && above > upper_right)
        || (above < upper_left && above < upper_right)
    {
        (right + left + 2 * above) >> 2
    } else {
        (left + right) >> 1
    };
    let code = read_code(reader, params, &mut gradients[gradient_index])?;
    if gradient_sign < 0 {
        prediction -= code;
    } else {
        prediction += code;
    }
    prediction = wrap_prediction(prediction, params);
    lines[line][1 + *position] = clamp_sample(prediction, params.max_value);
    *position += 2;
    Ok(())
}

fn interpolate_even(lines: &mut [Vec<u16>], line: usize, position: &mut usize) {
    let above = lines[line - 1][1 + *position] as i32;
    let left = lines[line - 1][*position] as i32;
    let right = lines[line - 1][2 + *position] as i32;
    let upper = lines[line - 2][1 + *position] as i32;
    let diff_left = (left - above).abs();
    let diff_upper = (upper - above).abs();
    let diff_right = (right - above).abs();
    lines[line][1 + *position] = if diff_left > diff_upper && diff_left > diff_right {
        ((upper + right + 2 * above) >> 2) as u16
    } else if diff_right > diff_left && diff_right > diff_upper {
        ((upper + left + 2 * above) >> 2) as u16
    } else {
        ((right + left + 2 * above) >> 2) as u16
    };
    *position += 2;
}

fn read_code(
    reader: &mut BitReader<'_>,
    params: &Params,
    gradient: &mut Gradient,
) -> Result<i32, CodecError> {
    let zeros = reader.zero_run()?;
    let special_threshold = params
        .max_bits
        .saturating_sub(q_raw_bits(params.total_values))
        .saturating_sub(1);
    let mut code = if zeros < special_threshold {
        let extra_bits = bit_difference(gradient.divisor, gradient.shift);
        (zeros << extra_bits) as i32 + reader.read_i32(extra_bits)?
    } else {
        1 + read_q_bits(reader, params.total_values)?
    };
    if code < 0 || code >= params.total_values {
        return Err(invalid(format!(
            "Fujifilm lossless code {code} is outside 0..{total} (zeros={zeros}, threshold={special_threshold}, gradient={gradient:?})",
            total = params.total_values,
            gradient = gradient
        )));
    }
    code = if code & 1 != 0 {
        -1 - code / 2
    } else {
        code / 2
    };
    gradient.divisor += code.abs();
    if gradient.shift == params.min_value {
        gradient.divisor >>= 1;
        gradient.shift >>= 1;
    }
    gradient.shift += 1;
    Ok(code)
}

fn q_raw_bits(total_values: i32) -> u32 {
    u32::try_from(total_values.max(1) - 1)
        .unwrap_or(u32::MAX)
        .ilog2()
        + 1
}

fn read_q_bits(reader: &mut BitReader<'_>, total_values: i32) -> Result<i32, CodecError> {
    reader.read_i32(q_raw_bits(total_values))
}

fn bit_difference(first: i32, second: i32) -> u32 {
    if second >= first {
        return 0;
    }
    for bits in 1..=15 {
        if (second << bits) >= first {
            return bits;
        }
    }
    15
}

const fn clamp_sample(value: i32, max: i32) -> u16 {
    if value < 0 {
        0
    } else if value > max {
        max as u16
    } else {
        value as u16
    }
}

const fn wrap_prediction(value: i32, params: &Params) -> i32 {
    if value < 0 {
        value + params.total_values
    } else if value > params.max_value {
        value - params.total_values
    } else {
        value
    }
}

fn shift_previous_lines(block: &mut Block) {
    for (destination, source) in [(R0, R3), (R1, R4), (G0, G6), (G1, G7), (B0, B3), (B1, B4)] {
        let (before, after) = block.lines.split_at_mut(source);
        before[destination].copy_from_slice(&after[0]);
    }
}

fn clear_current_lines(block: &mut Block, line_width: usize) {
    for (start, count) in [(R2, 3), (G2, 6), (B2, 3)] {
        for line in start..start + count {
            block.lines[line].fill(0);
        }
        block.lines[start][0] = block.lines[start - 1][1];
        block.lines[start][line_width + 1] = block.lines[start - 1][line_width];
    }
}

fn copy_lines(block: &Block, geometry: &BlockGeometry, block_line: usize, output: &mut [u16]) {
    let red = [R2, R3, R4];
    let green = [G2, G3, G4, G5, G6, G7];
    let blue = [B2, B3, B4];
    for row in 0..BLOCK_ROWS {
        let y = block_line * BLOCK_ROWS + row;
        let x = geometry.block_index * BLOCK_WIDTH;
        for pixel in 0..geometry.block_width {
            let line = match geometry.cfa.color_at(row, pixel) {
                0 => red[row >> 1],
                1 => green[row],
                2 => blue[row >> 1],
                _ => continue,
            };
            let index = if matches!(geometry.cfa, FujiCfa::XTrans(_)) {
                let thirds = pixel * 2 / 3;
                ((thirds & 0x7fff_fffe) | ((pixel % 3) & 1)) + ((pixel % 3) >> 1)
            } else {
                pixel >> 1
            };
            output[y * geometry.image_width + x + pixel] = block.lines[line][1 + index];
        }
    }
}

struct BlockGeometry {
    cfa: FujiCfa,
    block_index: usize,
    block_width: usize,
    image_width: usize,
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, bit: 0 } }

    fn read_bit(&mut self) -> Result<u32, CodecError> {
        let byte = self
            .bytes
            .get(self.bit / 8)
            .copied()
            .ok_or_else(|| invalid("Fujifilm lossless bitstream is truncated"))?;
        let value = u32::from((byte >> (7 - self.bit % 8)) & 1);
        self.bit += 1;
        Ok(value)
    }

    fn read_i32(&mut self, bits: u32) -> Result<i32, CodecError> {
        if bits > 31 {
            return Err(invalid("Fujifilm bit read exceeds 31 bits"));
        }
        let mut value = 0_u32;
        for _ in 0..bits {
            value = (value << 1) | self.read_bit()?;
        }
        Ok(value as i32)
    }

    fn zero_run(&mut self) -> Result<u32, CodecError> {
        let mut count = 0;
        loop {
            if self.read_bit()? == 1 {
                return Ok(count);
            }
            count = count
                .checked_add(1)
                .ok_or_else(|| invalid("Fujifilm zero run overflow"))?;
            if count > 256 {
                return Err(invalid("Fujifilm zero run is unreasonably long"));
            }
        }
    }
}

struct BigEndianReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BigEndianReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn u8(&mut self) -> Result<u8, CodecError> {
        let value = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| invalid("Fujifilm header is truncated"))?;
        self.position += 1;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, CodecError> {
        let end = self
            .position
            .checked_add(2)
            .ok_or_else(|| invalid("Fujifilm header offset overflow"))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| invalid("Fujifilm header is truncated"))?;
        self.position = end;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        let end = self
            .position
            .checked_add(4)
            .ok_or_else(|| invalid("Fujifilm header offset overflow"))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| invalid("Fujifilm header is truncated"))?;
        self.position = end;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_header;

    #[test]
    fn rounded_width_must_be_block_aligned() {
        let mut source = [0_u8; 16];
        source[0..2].copy_from_slice(&0x4953_u16.to_be_bytes());
        source[2] = 1;
        source[3] = 0;
        source[4] = 14;
        source[5..7].copy_from_slice(&6_u16.to_be_bytes());
        source[7..9].copy_from_slice(&769_u16.to_be_bytes());
        source[9..11].copy_from_slice(&768_u16.to_be_bytes());
        source[11..13].copy_from_slice(&768_u16.to_be_bytes());
        source[13] = 1;
        source[14..16].copy_from_slice(&1_u16.to_be_bytes());

        let error = parse_header(&source).unwrap_err();

        assert!(error.to_string().contains("geometry"));
    }
}
