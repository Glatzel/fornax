use fornax_raw_core::error::DecodeError;

use crate::container::{MakerNote, RawLayout, SENSOR_CORRECTION};
use crate::invalid;

const SENSOR_TEMPERATURE: u32 = 0x0210;
const SPLIT_COL: u32 = 0x0222;
const BLACK_COL: u32 = 0x0223;
const SPLIT_ROW: u32 = 0x0224;
const BLACK_ROW: u32 = 0x0225;

const DEFECTS: u32 = 0x0400;
const FLOAT_FLAT: u32 = 0x0401;
const CHROMA_FLAT: u32 = 0x040b;
const LUMA_FLAT: u32 = 0x0410;
const LUMA_FLAT_2: u32 = 0x0416;
const POLY_HALF: u32 = 0x0419;
const POLY_FULL: u32 = 0x041a;
const QUADRANT_MULTIPLIERS: u32 = 0x041e;
const QUADRANT_LINEARIZATION: u32 = 0x041f;
const QUADRANT_COMBINED: u32 = 0x0431;

const MAX_CORRECTION_ENTRIES: u32 = 8192;

pub fn apply(
    pixels: &mut [u16],
    maker: &MakerNote,
    layout: &RawLayout,
    cfa: [u8; 4],
) -> Result<(), DecodeError> {
    let expected = layout
        .width
        .checked_mul(layout.height)
        .ok_or_else(|| invalid("IIQ correction sample count overflow"))?;
    if pixels.len() != expected {
        return Err(invalid(format!(
            "IIQ correction received {} samples, expected {expected}",
            pixels.len()
        )));
    }
    let black = layout.black_level(maker)?;
    apply_black(pixels, layout, maker, black)?;
    let block = CorrectionBlock::parse(maker)?;
    for (tag, data) in block.entries.iter().copied() {
        if tag == POLY_HALF {
            apply_poly_half(pixels, layout, maker, data)?;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if tag == POLY_FULL {
            apply_poly_full(pixels, data)?;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if tag == QUADRANT_MULTIPLIERS {
            apply_quadrant_multipliers(pixels, layout, maker, data)?;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if tag == QUADRANT_COMBINED {
            apply_quadrant_combined(pixels, layout, maker, data)?;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if tag == QUADRANT_LINEARIZATION {
            apply_quadrant_linearization(pixels, layout, maker, data)?;
            break;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if tag == DEFECTS {
            apply_defects(pixels, layout.width, layout.height, cfa, data)?;
        }
    }
    for (tag, data) in block.entries.iter().copied() {
        if matches!(tag, FLOAT_FLAT | CHROMA_FLAT | LUMA_FLAT | LUMA_FLAT_2) {
            let (left, top) = layout.sensor_margins(maker)?;
            apply_flat(
                pixels,
                layout.width,
                layout.height,
                cfa,
                (left, top),
                tag,
                data,
            )?;
        }
    }
    Ok(())
}

fn apply_black(
    pixels: &mut [u16],
    layout: &RawLayout,
    maker: &MakerNote,
    black: u16,
) -> Result<(), DecodeError> {
    let split = split_blacks(maker, layout.width, layout.height)?;
    if let Some((columns, rows, split_col, split_row)) = split {
        for row in 0..layout.height {
            for column in 0..layout.width {
                let index = row * layout.width + column;
                let value = i32::from(pixels[index]) - i32::from(black)
                    + i32::from(columns[row * 2 + usize::from(column >= split_col)])
                    + i32::from(rows[column * 2 + usize::from(row >= split_row)]);
                pixels[index] = clamp(value);
            }
        }
    } else {
        for pixel in pixels {
            *pixel = clamp(i32::from(*pixel) - i32::from(black));
        }
    }
    Ok(())
}

fn split_blacks(
    maker: &MakerNote,
    width: usize,
    height: usize,
) -> Result<Option<(Vec<i16>, Vec<i16>, usize, usize)>, DecodeError> {
    let columns = maker.optional_u32(BLACK_COL)?;
    let rows = maker.optional_u32(BLACK_ROW)?;
    match (columns, rows) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => {
            let split_col = maker.required_u32(SPLIT_COL, "split column")?;
            let split_row = maker.required_u32(SPLIT_ROW, "split row")?;
            let split_col = usize::try_from(split_col)
                .map_err(|_| invalid("IIQ split column exceeds the address space"))?;
            let split_row = usize::try_from(split_row)
                .map_err(|_| invalid("IIQ split row exceeds the address space"))?;
            if split_col > width || split_row > height {
                return Err(invalid("IIQ split offsets are outside the raw image"));
            }
            let column_values = i16_values(maker.bytes(BLACK_COL)?, "column black offsets")?;
            let row_values = i16_values(maker.bytes(BLACK_ROW)?, "row black offsets")?;
            if column_values.len() != height * 2 || row_values.len() != width * 2 {
                return Err(invalid("IIQ split-black tables have invalid lengths"));
            }
            Ok(Some((column_values, row_values, split_col, split_row)))
        }
        _ => Err(invalid("IIQ split-black metadata is incomplete")),
    }
}

fn apply_poly_half(
    pixels: &mut [u16],
    layout: &RawLayout,
    maker: &MakerNote,
    data: &[u8],
) -> Result<(), DecodeError> {
    let mut polynomial = f32_array::<8>(data, "half polynomial")?;
    let temperature = maker
        .optional_u32(SENSOR_TEMPERATURE)?
        .ok_or_else(|| invalid("IIQ half polynomial requires a sensor temperature"))?;
    let temperature = f32::from_bits(temperature);
    polynomial[3] += (temperature - polynomial[7]) * polynomial[6] + 1.0;
    let split_col = usize::try_from(maker.required_u32(SPLIT_COL, "split column")?)
        .map_err(|_| invalid("IIQ split column exceeds the address space"))?;
    if split_col > layout.width {
        return Err(invalid("IIQ split column is outside the raw image"));
    }
    let curve: Vec<u16> = (0_u32..=u16::MAX as u32)
        .map(|value| {
            let value = value as f32;
            clamp_f32((polynomial[5] * value + polynomial[3]) * value + polynomial[1])
        })
        .collect();
    for row in 0..layout.height {
        for pixel in &mut pixels[row * layout.width + split_col..][..layout.width - split_col] {
            *pixel = curve[usize::from(*pixel)];
        }
    }
    Ok(())
}

fn apply_poly_full(pixels: &mut [u16], data: &[u8]) -> Result<(), DecodeError> {
    let polynomial = f32_array::<4>(data, "full polynomial")?;
    let curve: Vec<u16> = (0_u32..=u16::MAX as u32)
        .map(|value| {
            let mut result = 0.0_f32;
            for coefficient in polynomial.iter().rev() {
                result = result * value as f32 + coefficient;
            }
            clamp_f32(result + value as f32)
        })
        .collect();
    for pixel in pixels {
        *pixel = curve[usize::from(*pixel)];
    }
    Ok(())
}

fn apply_quadrant_multipliers(
    pixels: &mut [u16],
    layout: &RawLayout,
    maker: &MakerNote,
    data: &[u8],
) -> Result<(), DecodeError> {
    let multipliers = f32_array::<19>(data, "quadrant multipliers")?;
    let (split_col, split_row) = split_offsets(maker, layout)?;
    let gains = [
        1.0 + multipliers[4],
        1.0 + multipliers[10],
        1.0 + multipliers[14],
        1.0 + multipliers[18],
    ];
    for row in 0..layout.height {
        let row_gain = usize::from(row >= split_row) * 2;
        for column in 0..layout.width {
            let gain = gains[row_gain + usize::from(column >= split_col)];
            let index = row * layout.width + column;
            pixels[index] = clamp_f32(f32::from(pixels[index]) * gain);
        }
    }
    Ok(())
}

fn apply_quadrant_combined(
    pixels: &mut [u16],
    layout: &RawLayout,
    maker: &MakerNote,
    data: &[u8],
) -> Result<(), DecodeError> {
    if data.len() < 140 {
        return Err(invalid("IIQ combined quadrant data is truncated"));
    }
    let coordinates: [u16; 7] = std::array::from_fn(|index| {
        let start = index * 4;
        u32::from_le_bytes(data[start..start + 4].try_into().expect("four bytes")) as u16
    });
    let values: [u16; 28] = std::array::from_fn(|index| {
        let start = 28 + index * 4;
        u32::from_le_bytes(data[start..start + 4].try_into().expect("four bytes")) as u16
    });
    let (split_col, split_row) = split_offsets(maker, layout)?;
    for quadrant_row in 0..2 {
        for quadrant_col in 0..2 {
            let mut points = vec![(0_u16, 0_u16)];
            for index in 0..7 {
                points.push((
                    coordinates[index],
                    ((u32::from(coordinates[index])
                        * u32::from(values[quadrant_row * 14 + quadrant_col * 7 + index]))
                        / 10_000) as u16,
                ));
            }
            points.push((u16::MAX, u16::MAX));
            apply_curve_region(
                pixels,
                layout.width,
                layout.height,
                split_col,
                split_row,
                quadrant_row,
                quadrant_col,
                &linear_curve(&points)?,
            );
        }
    }
    Ok(())
}

fn apply_quadrant_linearization(
    pixels: &mut [u16],
    layout: &RawLayout,
    maker: &MakerNote,
    data: &[u8],
) -> Result<(), DecodeError> {
    if data.len() < 256 * 4 {
        return Err(invalid("IIQ quadrant linearization data is truncated"));
    }
    let mut tables = [[[0_u16; 16]; 2]; 2];
    for (index, table) in tables.iter_mut().flatten().flatten().enumerate() {
        let start = index * 4;
        *table = u32::from_le_bytes(data[start..start + 4].try_into().expect("four bytes")) as u16;
    }
    if tables
        .iter()
        .any(|quadrant| quadrant.iter().any(|table| table[15] == 0))
    {
        return Err(invalid(
            "IIQ quadrant linearization contains a zero control point",
        ));
    }
    let (split_col, split_row) = split_offsets(maker, layout)?;
    for quadrant_row in 0..2 {
        for quadrant_col in 0..2 {
            let table = tables[quadrant_row][quadrant_col];
            let mut reference = [0_u16; 16];
            for index in 0..16 {
                let total: u32 = tables
                    .iter()
                    .flatten()
                    .map(|table| u32::from(table[index]))
                    .sum();
                reference[index] = ((total + 2) >> 2) as u16;
            }
            let mut points = vec![(0_u16, 0_u16)];
            points.extend(table.into_iter().zip(reference));
            let scaled =
                ((u32::from(reference[15]) * u32::from(u16::MAX)) / u32::from(table[15])) as u16;
            points.push((scaled, scaled));
            points.push((u16::MAX, u16::MAX));
            apply_curve_region(
                pixels,
                layout.width,
                layout.height,
                split_col,
                split_row,
                quadrant_row,
                quadrant_col,
                &linear_curve(&points)?,
            );
        }
    }
    Ok(())
}

fn apply_curve_region(
    pixels: &mut [u16],
    width: usize,
    height: usize,
    split_col: usize,
    split_row: usize,
    quadrant_row: usize,
    quadrant_col: usize,
    curve: &[u16],
) {
    let (row_start, row_end) = if quadrant_row == 0 {
        (0, split_row)
    } else {
        (split_row, height)
    };
    let (column_start, column_end) = if quadrant_col == 0 {
        (0, split_col)
    } else {
        (split_col, width)
    };
    for row in row_start..row_end {
        for pixel in &mut pixels[row * width + column_start..][..column_end - column_start] {
            *pixel = curve[usize::from(*pixel)];
        }
    }
}

fn apply_defects(
    pixels: &mut [u16],
    width: usize,
    height: usize,
    cfa: [u8; 4],
    data: &[u8],
) -> Result<(), DecodeError> {
    if data.len() % 8 != 0 {
        return Err(invalid("IIQ defect table has a partial entry"));
    }
    for defect in data.as_chunks::<8>().0 {
        let column = usize::from(u16::from_le_bytes([defect[0], defect[1]]));
        let kind = u16::from_le_bytes([defect[4], defect[5]]);
        if column >= width || !matches!(kind, 131 | 137) {
            continue;
        }
        for row in 2..height.saturating_sub(2) {
            let index = row * width + column;
            match color_at(cfa, row, column) {
                1 => {
                    let values = [
                        pixels[(row - 1) * width + column - 1],
                        pixels[(row + 1) * width + column - 1],
                        pixels[(row - 1) * width + column + 1],
                        pixels[(row + 1) * width + column + 1],
                    ];
                    let sum: i32 = values.iter().map(|value| i32::from(*value)).sum();
                    let deviations = values.map(|value| (i32::from(value) * 4 - sum).abs());
                    let mut largest = 0;
                    for index in 1..deviations.len() {
                        if deviations[largest] < deviations[index] {
                            largest = index;
                        }
                    }
                    pixels[index] = ((sum - i32::from(values[largest]) + 1) / 3) as u16;
                }
                _ => {
                    let diagonals = u32::from(pixels[(row - 2) * width + column - 2])
                        + u32::from(pixels[(row - 2) * width + column + 2])
                        + u32::from(pixels[(row + 2) * width + column - 2])
                        + u32::from(pixels[(row + 2) * width + column + 2]);
                    let horizontal = u32::from(pixels[row * width + column - 2])
                        + u32::from(pixels[row * width + column + 2]);
                    pixels[index] = (diagonals as f32 * 0.073_223_3
                        + horizontal as f32 * 0.353_553_4)
                        .round() as u16;
                }
            }
        }
    }
    Ok(())
}

fn apply_flat(
    pixels: &mut [u16],
    width: usize,
    height: usize,
    cfa: [u8; 4],
    margins: (usize, usize),
    tag: u32,
    data: &[u8],
) -> Result<(), DecodeError> {
    if data.len() < 16 {
        return Err(invalid(format!(
            "IIQ flat-field tag 0x{tag:04x} is truncated"
        )));
    }
    let head: [u16; 4] =
        std::array::from_fn(|index| u16::from_le_bytes([data[index * 2], data[index * 2 + 1]]));
    let [start_x, start_y, flat_width, flat_height] = head.map(usize::from);
    let step_x = usize::from(u16::from_le_bytes([data[8], data[9]]));
    let step_y = usize::from(u16::from_le_bytes([data[10], data[11]]));
    if step_x == 0 || step_y == 0 || flat_width == 0 || flat_height == 0 {
        return Ok(());
    }
    let colors = if tag == CHROMA_FLAT { 4 } else { 2 };
    let wide = flat_width.div_ceil(step_x);
    let high = flat_height.div_ceil(step_y);
    let count = high
        .checked_mul(wide)
        .and_then(|count| count.checked_mul(colors / 2))
        .ok_or_else(|| invalid("IIQ flat-field table size overflow"))?;
    let mut values = Vec::with_capacity(count);
    if tag == FLOAT_FLAT {
        if data.len() < 16 + count * 4 {
            return Err(invalid("IIQ floating-point flat field is truncated"));
        }
        for index in 0..count {
            let start = 16 + index * 4;
            values.push(f32::from_le_bytes(
                data[start..start + 4].try_into().expect("four bytes"),
            ));
        }
    } else {
        if data.len() < 16 + count * 2 {
            return Err(invalid("IIQ integer flat field is truncated"));
        }
        for index in 0..count {
            let start = 16 + index * 2;
            values.push(f32::from(u16::from_le_bytes([data[start], data[start + 1]])) / 32768.0);
        }
    }
    let mut rows = vec![0.0_f32; colors * wide];
    let mut value = values.into_iter();
    let mut multipliers = [0.0_f32; 4];
    for y in 0..high {
        for x in 0..wide {
            for color in (0..colors).step_by(2) {
                let next = value
                    .next()
                    .ok_or_else(|| invalid("IIQ flat field consumes beyond its table"))?;
                if y == 0 {
                    rows[color * wide + x] = next;
                } else {
                    rows[(color + 1) * wide + x] = (next - rows[color * wide + x]) / step_y as f32;
                }
            }
        }
        if y == 0 {
            continue;
        }
        let row_end = start_y + y * step_y;
        let row_limit = start_y
            .checked_add(flat_height)
            .and_then(|end| end.checked_sub(step_y))
            .ok_or_else(|| invalid("IIQ flat-field row range underflow"))?;
        for row in (row_end - step_y)..row_end.min(height).min(row_limit) {
            for x in 1..wide {
                for color in (0..colors).step_by(2) {
                    multipliers[color] = rows[color * wide + x - 1];
                    multipliers[color + 1] =
                        (rows[color * wide + x] - multipliers[color]) / step_x as f32;
                }
                let column_end = start_x + x * step_x;
                let column_limit = start_x
                    .checked_add(flat_width)
                    .and_then(|end| end.checked_sub(step_x))
                    .ok_or_else(|| invalid("IIQ flat-field column range underflow"))?;
                for column in (column_end - step_x)..column_end.min(width).min(column_limit) {
                    let color = if colors > 2 {
                        if row < margins.1 || column < margins.0 {
                            return Err(invalid("IIQ flat field begins inside the sensor margin"));
                        }
                        usize::from(color_at(cfa, row - margins.1, column - margins.0))
                    } else {
                        0
                    };
                    if color & 1 == 0 {
                        let index = row * width + column;
                        pixels[index] = clamp_f32(f32::from(pixels[index]) * multipliers[color]);
                    }
                    for color in (0..colors).step_by(2) {
                        multipliers[color] += multipliers[color + 1];
                    }
                }
            }
            for x in 0..wide {
                for color in (0..colors).step_by(2) {
                    rows[color * wide + x] += rows[(color + 1) * wide + x];
                }
            }
        }
    }
    Ok(())
}

fn split_offsets(maker: &MakerNote, layout: &RawLayout) -> Result<(usize, usize), DecodeError> {
    let split_col = usize::try_from(maker.required_u32(SPLIT_COL, "split column")?)
        .map_err(|_| invalid("IIQ split column exceeds the address space"))?;
    let split_row = usize::try_from(maker.required_u32(SPLIT_ROW, "split row")?)
        .map_err(|_| invalid("IIQ split row exceeds the address space"))?;
    if split_col > layout.width || split_row > layout.height {
        return Err(invalid("IIQ split offsets are outside the raw image"));
    }
    Ok((split_col, split_row))
}

fn linear_curve(points: &[(u16, u16)]) -> Result<Vec<u16>, DecodeError> {
    if points.len() < 2
        || points.first() != Some(&(0, 0))
        || points.last() != Some(&(u16::MAX, u16::MAX))
    {
        return Err(invalid("IIQ correction curve endpoints are invalid"));
    }
    let mut curve = vec![0_u16; usize::from(u16::MAX) + 1];
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if x1 < x0 {
            return Err(invalid("IIQ correction curve coordinates are unordered"));
        }
        if x0 == x1 {
            continue;
        }
        let slope = (f32::from(y1) - f32::from(y0)) / (f32::from(x1) - f32::from(x0));
        for x in x0..=x1 {
            curve[usize::from(x)] = (f32::from(y0) + slope * f32::from(x - x0)) as u16;
        }
    }
    Ok(curve)
}

fn f32_array<const N: usize>(data: &[u8], name: &str) -> Result<[f32; N], DecodeError> {
    if data.len() < N * 4 {
        return Err(invalid(format!("IIQ {name} is truncated")));
    }
    Ok(std::array::from_fn(|index| {
        let start = index * 4;
        f32::from_le_bytes(data[start..start + 4].try_into().expect("four bytes"))
    }))
}

fn i16_values(data: &[u8], name: &str) -> Result<Vec<i16>, DecodeError> {
    if !data.len().is_multiple_of(2) {
        return Err(invalid(format!("IIQ {name} contain a partial value")));
    }
    Ok(data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| i16::from_le_bytes(*bytes))
        .collect())
}

const fn color_at(cfa: [u8; 4], row: usize, column: usize) -> u8 {
    cfa[(row & 1) * 2 + (column & 1)]
}

fn clamp(value: i32) -> u16 { value.clamp(0, i32::from(u16::MAX)) as u16 }

fn clamp_f32(value: f32) -> u16 { value.clamp(0.0, f32::from(u16::MAX)) as u16 }

struct CorrectionBlock<'a> {
    entries: Vec<(u32, &'a [u8])>,
}

impl<'a> CorrectionBlock<'a> {
    fn parse(maker: &'a MakerNote) -> Result<Self, DecodeError> {
        let Ok(block) = maker.bytes(SENSOR_CORRECTION) else {
            return Ok(Self {
                entries: Vec::new(),
            });
        };
        if block.len() < 12 {
            return Err(invalid("IIQ sensor correction header is truncated"));
        }
        let entries_relative =
            u32::from_le_bytes(block[8..12].try_into().expect("four bytes")) as usize;
        let entries_start = entries_relative;
        let count = u32::from_le_bytes(
            block
                .get(entries_start..entries_start + 4)
                .ok_or_else(|| invalid("IIQ sensor correction table is truncated"))?
                .try_into()
                .expect("four bytes"),
        );
        if count > MAX_CORRECTION_ENTRIES {
            return Err(invalid(format!(
                "IIQ sensor correction entry count {count} exceeds {MAX_CORRECTION_ENTRIES}"
            )));
        }
        let table_start = entries_start + 8;
        let table = block
            .get(
                table_start
                    ..table_start
                        + usize::try_from(count)
                            .ok()
                            .and_then(|count| count.checked_mul(12))
                            .ok_or_else(|| invalid("IIQ correction table size overflow"))?,
            )
            .ok_or_else(|| invalid("IIQ sensor correction table is truncated"))?;
        let mut entries = Vec::with_capacity(count as usize);
        for entry in table.as_chunks::<12>().0 {
            let tag = u32::from_le_bytes(entry[0..4].try_into().expect("four bytes"));
            if !matches!(
                tag,
                DEFECTS
                    | FLOAT_FLAT
                    | CHROMA_FLAT
                    | LUMA_FLAT
                    | LUMA_FLAT_2
                    | POLY_HALF
                    | POLY_FULL
                    | QUADRANT_MULTIPLIERS
                    | QUADRANT_LINEARIZATION
                    | QUADRANT_COMBINED
            ) {
                continue;
            }
            let length = u32::from_le_bytes(entry[4..8].try_into().expect("four bytes")) as usize;
            let offset = u32::from_le_bytes(entry[8..12].try_into().expect("four bytes")) as usize;
            let end = offset
                .checked_add(length)
                .ok_or_else(|| invalid("IIQ correction entry range overflow"))?;
            let data = block.get(offset..end).ok_or_else(|| {
                invalid(format!("IIQ correction tag 0x{tag:04x} is out of bounds"))
            })?;
            entries.push((tag, data));
        }
        Ok(Self { entries })
    }
}
