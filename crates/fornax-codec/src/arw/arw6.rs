use rayon::prelude::*;

use super::llvc3::LLVC3_CURVE;
use crate::error::invalid;
use crate::request::SampleFormat;
use crate::samples::{buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, SampleBuffer};

const STREAM_TABLE_START: usize = 0x200;
const BIAS: i32 = 2048;
const MAX_PLANE_SAMPLES: usize = 128 * 1024 * 1024;

fn error(detail: impl Into<String>) -> CodecError {
    invalid(format!("Sony ARW6: {}", detail.into()))
}

fn require(condition: bool, detail: &str) -> Result<(), CodecError> {
    if condition {
        Ok(())
    } else {
        Err(error(detail))
    }
}

#[derive(Clone, Default)]
struct Plane {
    rows: usize,
    columns: usize,
    samples: Vec<i32>,
}

impl Plane {
    fn new(rows: usize, columns: usize) -> Result<Self, CodecError> {
        let count = rows
            .checked_mul(columns)
            .filter(|count| *count <= MAX_PLANE_SAMPLES)
            .ok_or_else(|| error("plane dimensions overflow"))?;
        Ok(Self {
            rows,
            columns,
            samples: vec![0; count],
        })
    }

    fn at(&self, row: usize, column: usize) -> i32 { self.samples[row * self.columns + column] }

    fn set(&mut self, row: usize, column: usize, value: i32) {
        self.samples[row * self.columns + column] = value;
    }

    fn row(&self, row: usize) -> &[i32] {
        &self.samples[row * self.columns..(row + 1) * self.columns]
    }

    fn row_mut(&mut self, row: usize) -> &mut [i32] {
        &mut self.samples[row * self.columns..(row + 1) * self.columns]
    }

    fn first_rows(&self, rows: usize) -> Result<Self, CodecError> {
        require(rows <= self.rows, "plane row count is invalid")?;
        Ok(Self {
            rows,
            columns: self.columns,
            samples: self.samples[..rows * self.columns].to_vec(),
        })
    }
}

#[inline]
fn floor_shift(value: i32, bits: u32) -> i32 { value >> bits }

#[inline]
fn sign_extend_16(value: i32) -> i32 { value as u16 as i16 as i32 }

#[inline]
fn edge_detail(value: i32, odd_mode: bool) -> i32 {
    let half = value.signum();
    let odd = value % 2 != 0;
    let add_half = if odd_mode { odd } else { value != 0 && !odd };
    value
        .wrapping_mul(2)
        .wrapping_add(if add_half { half } else { 0 })
}

struct HeaderBits<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl HeaderBits<'_> {
    fn read(&mut self, count: u32) -> Result<u32, CodecError> {
        require(
            self.position + count as usize <= self.bytes.len() * 8,
            "packet header exceeds data",
        )?;
        let mut value = 0_u32;
        for _ in 0..count {
            let byte = self.bytes[self.position >> 3];
            value = (value << 1) | u32::from((byte >> (7 - (self.position & 7))) & 1);
            self.position += 1;
        }
        Ok(value)
    }
}

struct CoeffBits<'a> {
    bytes: &'a [u8],
    next_byte: usize,
    cache: u64,
    cached_bits: u32,
    remaining_bits: u64,
    status: u8,
}

impl<'a> CoeffBits<'a> {
    fn new(bytes: &'a [u8], offset: usize, length: usize) -> Self {
        let words = ((length.div_ceil(8)) + 1) & !1;
        Self {
            bytes,
            next_byte: offset,
            cache: 0,
            cached_bits: 0,
            remaining_bits: words as u64 * 64,
            status: 0,
        }
    }

    fn refill(&mut self) {
        if self.cached_bits > 55 {
            return;
        }
        let bytes_to_load = (63 - self.cached_bits) >> 3;
        let mut block = [0_u8; 8];
        if let Some(bytes) = self.bytes.get(self.next_byte..self.next_byte + 8) {
            block.copy_from_slice(bytes);
        } else if let Some(bytes) = self.bytes.get(self.next_byte..) {
            let count = bytes.len().min(8);
            block[..count].copy_from_slice(&bytes[..count]);
        }
        let loaded_bits = bytes_to_load * 8;
        let value = u64::from_be_bytes(block);
        self.cache |= (value >> (64 - loaded_bits)) << (64 - loaded_bits - self.cached_bits);
        self.next_byte += bytes_to_load as usize;
        self.cached_bits += loaded_bits;
    }

    fn consume(&mut self, count: u32) {
        self.cache <<= count;
        self.cached_bits -= count;
        self.remaining_bits -= u64::from(count);
    }

    fn read(&mut self, count: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        if u64::from(count) > self.remaining_bits {
            return self.read_past_limit(count);
        }
        if self.cached_bits < count {
            self.refill();
        }
        let value = (self.cache >> (64 - count)) as u32;
        self.consume(count);
        value
    }

    fn read_past_limit(&mut self, count: u32) -> u32 {
        let available = self.remaining_bits as u32;
        let head = if available > 0 {
            self.read(available)
        } else {
            0
        };
        self.status = 2;
        self.remaining_bits = 0;
        head.checked_shl(count - available).unwrap_or(0)
    }

    fn bit(&mut self) -> u32 { self.read(1) }

    fn unary_plus_one(&mut self) -> i32 {
        let mut zeros = 0_i32;
        loop {
            if self.status != 0 {
                return zeros + 1;
            }
            if self.remaining_bits == 0 {
                self.status = 2;
                return zeros + 1;
            }
            self.refill();
            let available = self.cached_bits.min(self.remaining_bits as u32);
            let zeros_in_cache = self.cache.leading_zeros();
            if zeros_in_cache < available {
                self.consume(zeros_in_cache + 1);
                return zeros + zeros_in_cache as i32 + 1;
            }
            self.consume(available);
            zeros += available as i32;
        }
    }

    fn leading_one_within(&mut self, count: u32) -> Option<u32> {
        if u64::from(count) <= self.remaining_bits && count <= 32 {
            self.refill();
            let zeros = self.cache.leading_zeros();
            if zeros < count {
                self.consume(zeros + 1);
                return Some(zeros);
            }
            self.consume(count);
            return None;
        }
        (0..count).find(|_| self.bit() != 0)
    }
}

fn update_width(bits: &mut CoeffBits<'_>, width: i32) -> i32 {
    if bits.bit() == 0 || bits.status != 0 {
        return width;
    }
    if bits.bit() == 0 || bits.status != 0 {
        return width + bits.unary_plus_one();
    }
    match bits.leading_one_within((width - 1).max(0) as u32) {
        Some(zeros) => width - (zeros as i32 + 1),
        None => 0,
    }
}

fn zero_run(bits: &mut CoeffBits<'_>, remaining: i32) -> i32 {
    if remaining <= 1 {
        return remaining;
    }
    let max_prefix = 32 - ((remaining - 1) as u32).leading_zeros() as i32;
    let mut zeros = 0;
    while zeros < max_prefix {
        if bits.bit() != 0 {
            break;
        }
        if bits.status != 0 {
            return remaining;
        }
        zeros += 1;
    }
    if zeros >= max_prefix {
        return remaining;
    }
    let base = 1 << zeros;
    if base >= remaining {
        return remaining;
    }
    let extra = if zeros > 0 {
        bits.read(zeros as u32) as i32
    } else {
        0
    };
    (base + extra).min(remaining)
}

fn decode_component(bits: &mut CoeffBits<'_>, groups: usize, shift: u32, output: &mut [i32]) {
    let row_width = output.len();
    let mut width = update_width(bits, 0);
    let mut group_index = 0;
    let mut output_index = 0;
    while group_index < groups {
        if bits.status != 0 || width > 0x13 {
            if bits.status == 0 {
                bits.status = 1;
            }
            output[output_index..].fill(0);
            return;
        }
        if width == 0 {
            let run = zero_run(bits, (groups - group_index) as i32) as usize;
            let count = (run * 4).min(row_width - output_index);
            output[output_index..output_index + count].fill(0);
            output_index += count;
            group_index += run;
            if group_index >= groups {
                break;
            }
            width = bits.unary_plus_one();
            continue;
        }
        let mut values = [0_i32; 4];
        for value in &mut values {
            let raw = bits.read(width as u32);
            *value = if shift > 0 && raw > 0 {
                (((2 * raw as u64 + 1) << (shift - 1)) - (raw as u64 & 1)) as i32
            } else {
                raw as i32
            };
        }
        let next_width = if group_index + 1 < groups {
            update_width(bits, width)
        } else {
            width
        };
        for value in &mut values {
            if *value > 0 {
                let sign = bits.bit() as i32;
                *value = value.wrapping_sub(2_i32.wrapping_mul(*value).wrapping_mul(sign));
            }
        }
        for value in values {
            if output_index < row_width {
                output[output_index] = value;
                output_index += 1;
            }
        }
        width = next_width;
        group_index += 1;
    }
    output[output_index..].fill(0);
}

#[derive(Clone, Copy)]
struct TileStream {
    offset: usize,
    length: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct DirectoryEntry {
    group: usize,
    index: usize,
    start: usize,
    length: usize,
}

struct Packet<'a> {
    kind: usize,
    bytes: &'a [u8],
    records: Vec<Record>,
}

struct Record {
    byte_length: usize,
    selectors: [u8; 3],
    payload: usize,
}

fn be16(bytes: &[u8], offset: usize) -> Result<usize, CodecError> {
    bytes
        .get(offset..offset + 2)
        .map(|value| usize::from(u16::from_be_bytes([value[0], value[1]])))
        .ok_or_else(|| error("truncated stream header"))
}

fn be32(bytes: &[u8], offset: usize) -> Result<u32, CodecError> {
    bytes
        .get(offset..offset + 4)
        .map(|value| u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
        .ok_or_else(|| error("truncated stream header"))
}

fn le32(bytes: &[u8], offset: usize) -> Result<u32, CodecError> {
    bytes
        .get(offset..offset + 4)
        .map(|value| u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
        .ok_or_else(|| error("truncated stream table"))
}

fn parse_stream_header(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 0x80 || !(bytes.starts_with(b"A000") || bytes.starts_with(b"0000")) {
        return None;
    }
    let width_code = be16(bytes, 0x0c).ok()?;
    let format_code = be16(bytes, 0x0e).ok()?;
    if (width_code >> 4) & 0x3f != 16 || format_code >> 13 != 3 || (format_code >> 10) & 3 != 3 {
        return None;
    }
    let width = be16(bytes, 8).ok()?;
    let half_height = be16(bytes, 0x0a).ok()?;
    (width != 0 && half_height != 0 && width % 16 == 0).then_some((width, half_height))
}

fn group_lengths(bytes: &[u8], offset: usize) -> Result<[usize; 5], CodecError> {
    let mut lengths = [0_usize; 5];
    for (index, length) in lengths.iter_mut().enumerate() {
        *length = ((be32(bytes, offset + index * 3)? >> 4) & 0x0fff_fff0) as usize;
    }
    Ok(lengths)
}

fn stream_length(bytes: &[u8]) -> Result<usize, CodecError> {
    require(bytes.len() >= 0x80, "stream is too short")?;
    let payload = group_lengths(bytes, 0x10)?.into_iter().sum::<usize>();
    require(
        payload <= bytes.len() - 0x80,
        "stream payload exceeds strip",
    )?;
    Ok(0x80 + payload)
}

fn locate_streams(strip: &[u8], full_width: usize, full_height: usize) -> Vec<TileStream> {
    if full_width == 0 || full_height == 0 || strip.len() < STREAM_TABLE_START + 0x80 {
        return Vec::new();
    }
    let declared = le32(strip, 0).unwrap_or(0) as usize;
    let mut streams = Vec::new();
    if (1..=16).contains(&declared) {
        for index in 0..declared {
            let entry = 0x08 + index * 0x18;
            let mut fields = [0_u32; 6];
            let mut valid = true;
            for (field, value) in fields.iter_mut().enumerate() {
                match le32(strip, entry + field * 4) {
                    Ok(field_value) => *value = field_value,
                    Err(_) => {
                        valid = false;
                        break;
                    }
                }
            }
            if !valid {
                streams.clear();
                break;
            }
            let table_offset = fields[0] as usize;
            let tile_x = fields[2] as usize;
            let tile_y = fields[3] as usize;
            let tile_width = fields[4] as usize;
            let tile_height = fields[5] as usize;
            let start = if table_offset == 0 {
                STREAM_TABLE_START
            } else {
                table_offset
            };
            if start > strip.len().saturating_sub(0x80) {
                streams.clear();
                break;
            }
            let end = (strip.len() - 0x80).min(start.saturating_add(0x1000));
            let mut candidate = start;
            let mut found = false;
            while candidate <= end {
                if let Some((width, half_height)) = parse_stream_header(&strip[candidate..])
                    && let Ok(length) = stream_length(&strip[candidate..])
                {
                    let height = half_height * 2;
                    let expected_width = if tile_width != 0 { tile_width } else { width };
                    let expected_height = if tile_height != 0 {
                        tile_height
                    } else {
                        height
                    };
                    if length > 0x80
                        && candidate + length <= strip.len()
                        && expected_width == width
                        && expected_height == height
                        && tile_x + width <= full_width
                        && tile_y + height <= full_height
                    {
                        streams.push(TileStream {
                            offset: candidate,
                            length,
                            x: tile_x,
                            y: tile_y,
                            width,
                            height,
                        });
                        found = true;
                        break;
                    }
                }
                candidate += 0x10;
            }
            if !found {
                streams.clear();
                break;
            }
        }
        if streams.len() == declared {
            return streams;
        }
        streams.clear();
    }
    if let Some((width, half_height)) = parse_stream_header(&strip[STREAM_TABLE_START..])
        && let Ok(length) = stream_length(&strip[STREAM_TABLE_START..])
        && width == full_width
        && half_height * 2 == full_height
    {
        streams.push(TileStream {
            offset: STREAM_TABLE_START,
            length,
            x: 0,
            y: 0,
            width,
            height: half_height * 2,
        });
    }
    streams
}

fn parse_directory(stream: &[u8]) -> Result<Vec<DirectoryEntry>, CodecError> {
    let groups = group_lengths(stream, 0x10)?;
    let mut entries = Vec::new();
    let mut position = 0x30;
    let mut group_start = 0;
    for (group, group_length) in groups.into_iter().enumerate() {
        require(
            position + 0x10 <= stream.len(),
            "packet directory exceeds stream",
        )?;
        let count = usize::from(stream[position] & 0x0f);
        require(count <= 9, "packet directory count is invalid")?;
        let mut values = [0_usize; 9];
        for (index, value) in values.iter_mut().take(5).enumerate() {
            *value = ((be32(stream, position + index * 3)? & 0x00ff_ffff) << 4) as usize;
        }
        let mut consumed = 0x10;
        if count >= 5 {
            require(
                position + 0x20 <= stream.len(),
                "packet directory exceeds stream",
            )?;
            for index in 0..4 {
                values[5 + index] =
                    ((be32(stream, position + 0x10 + index * 3)? & 0x00ff_ffff) << 4) as usize;
            }
            consumed = 0x20;
        }
        let mut local = 0;
        for (index, length) in values.into_iter().take(count).enumerate() {
            let start = 0x80 + group_start + local;
            require(
                start <= stream.len() && length <= stream.len() - start,
                "packet exceeds stream",
            )?;
            entries.push(DirectoryEntry {
                group,
                index,
                start,
                length,
            });
            local += length;
        }
        require(local <= group_length, "packet group size is invalid")?;
        group_start += group_length;
        position += consumed;
    }
    Ok(entries)
}

fn parse_packet(bytes: &[u8]) -> Result<Packet<'_>, CodecError> {
    require(bytes.len() >= 16, "packet is too short")?;
    let mut bits = HeaderBits { bytes, position: 0 };
    let control_count = bits.read(16)? as usize;
    let extra_count = bits.read(24)? as usize;
    let tag = bits.read(4)?;
    let reserved = bits.read(4)?;
    let kind = bits.read(2)? as usize;
    let control_words = bits.read(6)?;
    let record_count = bits.read(16)? as usize;
    let width_marker = bits.read(8)?;
    let reserved_byte = bits.read(8)?;
    for _ in 0..5 {
        require(bits.read(8)? == 0, "packet header padding is invalid")?;
    }
    require(
        tag == 4 && reserved == 0 && control_words == 0,
        "packet header tag is invalid",
    )?;
    require(kind == 1 || kind == 3, "packet type is unsupported")?;
    require(
        record_count > 0 && record_count <= 30_000,
        "packet record count is invalid",
    )?;
    require(
        width_marker == 0x10 && reserved_byte == 0,
        "packet width marker is invalid",
    )?;
    let control_bytes = (control_count + 1) << 4;
    let total_bytes = (control_count + 1 + extra_count) << 4;
    require(
        total_bytes == bytes.len() && control_bytes <= bytes.len(),
        "packet size is invalid",
    )?;
    let mut cursor = control_bytes;
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let byte_length = bits.read(16)? as usize;
        let mut selectors = [0_u8; 3];
        for selector in selectors.iter_mut().take(kind) {
            *selector = bits.read(4)? as u8;
        }
        require(
            byte_length <= bytes.len() && cursor <= bytes.len() - byte_length,
            "packet record exceeds packet",
        )?;
        records.push(Record {
            byte_length,
            selectors,
            payload: cursor,
        });
        cursor += byte_length;
    }
    Ok(Packet {
        kind,
        bytes,
        records,
    })
}

fn row_multiplier(group: usize) -> usize {
    match group {
        4 => 8,
        0 => 1,
        group => 1 << (group - 1),
    }
}

fn decode_packet_planes(
    stream: &[u8],
    directory: &[DirectoryEntry],
    mosaic_width: usize,
    coded_height: usize,
    group: usize,
    index: usize,
) -> Result<Vec<Plane>, CodecError> {
    let entry = directory
        .iter()
        .find(|entry| entry.group == group && entry.index == index)
        .ok_or_else(|| error("packet directory entry is missing"))?;
    let packet = parse_packet(&stream[entry.start..entry.start + entry.length])?;
    let components = if packet.kind == 1 { 1 } else { 3 };
    let row_width = match (packet.kind, group) {
        (1, 4) => mosaic_width / 2,
        (1, _) => mosaic_width / 16,
        (3, 1..=3) => mosaic_width >> (5 - group),
        _ => return Err(error("invalid packet type or group")),
    };
    let multiplier = row_multiplier(group);
    let rows = packet.records.len() * multiplier;
    require(
        coded_height > 0 && coded_height <= 0x7fff,
        "coded height is invalid",
    )?;
    let expected_rows = (coded_height.div_ceil(16) + 1) * multiplier;
    let groups_per_row = row_width.div_ceil(4);
    require(
        row_width > 0 && groups_per_row > 0 && rows > 0,
        "packet geometry is invalid",
    )?;
    if rows != expected_rows {
        require(
            !coded_height.is_multiple_of(16) && expected_rows.checked_sub(rows) == Some(multiplier),
            "packet row count is invalid",
        )?;
    }
    require(
        packet.records.len() <= stream.len() / 16 + 1,
        "packet record count is invalid",
    )?;
    let mut planes = (0..components)
        .map(|_| Plane::new(expected_rows, row_width))
        .collect::<Result<Vec<_>, _>>()?;
    let record_rows = multiplier * row_width;
    let decoded = packet
        .records
        .par_iter()
        .map(|record| {
            let mut values = vec![0_i32; components * record_rows];
            if record.byte_length > 0 {
                let mut bits = CoeffBits::new(packet.bytes, record.payload, record.byte_length);
                for component in 0..components {
                    let shift = if component < packet.kind {
                        u32::from(record.selectors[component])
                    } else {
                        0
                    };
                    for row in 0..multiplier {
                        let start = component * record_rows + row * row_width;
                        decode_component(
                            &mut bits,
                            groups_per_row,
                            shift,
                            &mut values[start..start + row_width],
                        );
                    }
                }
            }
            values
        })
        .collect::<Vec<_>>();
    for (record_index, values) in decoded.iter().enumerate() {
        for (component, plane) in planes.iter_mut().enumerate() {
            let destination =
                &mut plane.samples[record_index * record_rows..(record_index + 1) * record_rows];
            destination
                .copy_from_slice(&values[component * record_rows..(component + 1) * record_rows]);
        }
    }
    Ok(planes)
}

fn integrate(band_zero: &Plane, rows: usize, start: usize) -> Result<Plane, CodecError> {
    require(
        rows <= band_zero.rows && start <= rows,
        "band-zero rows are invalid",
    )?;
    let mut output = Plane::new(rows - start, band_zero.columns)?;
    for row in start..rows {
        let mut accumulator = 0_i32;
        for (destination, source) in output
            .row_mut(row - start)
            .iter_mut()
            .zip(band_zero.row(row))
        {
            accumulator = accumulator.wrapping_add(sign_extend_16(*source).wrapping_mul(2));
            *destination = accumulator / 2;
        }
    }
    Ok(output)
}

fn inverse_vertical(low: &Plane, high: &Plane) -> Result<Plane, CodecError> {
    require(
        low.rows == high.rows && low.columns == high.columns,
        "vertical lifting shapes are invalid",
    )?;
    let (rows, columns) = (low.rows, low.columns);
    let mut low_even = Plane::new(rows, columns)?;
    low_even
        .samples
        .par_chunks_mut(columns)
        .enumerate()
        .for_each(|(row, destination)| {
            let previous = high.row(row.saturating_sub(1));
            let current = high.row(row);
            for (column, value) in destination.iter_mut().enumerate() {
                *value = low.row(row)[column].wrapping_sub(floor_shift(
                    previous[column]
                        .wrapping_add(current[column])
                        .wrapping_add(2),
                    2,
                ));
            }
        });
    let mut output = Plane::new(rows * 2, columns)?;
    output
        .samples
        .par_chunks_mut(2 * columns)
        .enumerate()
        .for_each(|(row, pair)| {
            let next = low_even.row(if row + 1 < rows { row + 1 } else { row });
            let current = low_even.row(row);
            let high = high.row(row);
            let (even, odd) = pair.split_at_mut(columns);
            even.copy_from_slice(current);
            for (column, value) in odd.iter_mut().enumerate() {
                *value = high[column]
                    .wrapping_add(floor_shift(current[column].wrapping_add(next[column]), 1));
            }
        });
    Ok(output)
}

fn inverse_horizontal(low: &Plane, high: &Plane) -> Result<Plane, CodecError> {
    require(
        low.rows == high.rows && low.columns == high.columns,
        "horizontal lifting shapes are invalid",
    )?;
    let (rows, columns) = (low.rows, low.columns);
    let mut output = Plane::new(rows, columns * 2)?;
    if columns == 0 {
        return Ok(output);
    }
    output
        .samples
        .par_chunks_mut(2 * columns)
        .enumerate()
        .for_each(|(row, destination)| {
            let low = low.row(row);
            let high = high.row(row);
            let mut even = vec![0_i32; columns];
            even[0] = low[0].wrapping_sub(floor_shift(
                high[0].wrapping_add(high[0]).wrapping_add(2),
                2,
            ));
            for column in 1..columns {
                even[column] = low[column].wrapping_sub(floor_shift(
                    high[column - 1].wrapping_add(high[column]).wrapping_add(2),
                    2,
                ));
            }
            for column in 0..columns {
                let next = even[if column + 1 < columns {
                    column + 1
                } else {
                    column
                }];
                destination[2 * column] = even[column];
                destination[2 * column + 1] =
                    high[column].wrapping_add(floor_shift(even[column].wrapping_add(next), 1));
            }
        });
    Ok(output)
}

fn inverse_vertical_high_first(low: &Plane, high: &Plane) -> Result<Plane, CodecError> {
    require(low.columns == high.columns, "lifting shapes are invalid")?;
    require(
        high.rows == low.rows || high.rows == low.rows + 1,
        "lifting row counts are invalid",
    )?;
    require(low.rows > 0, "lifting band is empty")?;
    let (low_rows, high_rows, columns) = (low.rows, high.rows, low.columns);
    let mut low_even = Plane::new(low_rows, columns)?;
    let mut high_result = Plane::new(high_rows, columns)?;
    for row in 0..low_rows {
        let next = if high_rows == low_rows + 1 || row + 1 < high_rows {
            row + 1
        } else {
            row
        }
        .min(high_rows - 1);
        for column in 0..columns {
            let value = high
                .at(row, column)
                .wrapping_add(high.at(next, column))
                .wrapping_add(2);
            low_even.set(
                row,
                column,
                low.at(row, column).wrapping_sub(floor_shift(value, 2)),
            );
        }
    }
    if high_rows == low_rows + 1 {
        for column in 0..columns {
            high_result.set(
                0,
                column,
                high.at(0, column).wrapping_add(low_even.at(0, column)),
            );
            high_result.set(
                high_rows - 1,
                column,
                high.at(high_rows - 1, column)
                    .wrapping_add(low_even.at(low_rows - 1, column)),
            );
        }
        for row in 1..high_rows - 1 {
            for column in 0..columns {
                let value = floor_shift(
                    low_even
                        .at(row - 1, column)
                        .wrapping_add(low_even.at(row, column)),
                    1,
                );
                high_result.set(row, column, high.at(row, column).wrapping_add(value));
            }
        }
        let mut output = Plane::new(low_rows * 2 + 1, columns)?;
        for row in 0..low_rows {
            output
                .row_mut(2 * row)
                .copy_from_slice(high_result.row(row));
            output
                .row_mut(2 * row + 1)
                .copy_from_slice(low_even.row(row));
        }
        let last = output.rows - 1;
        output
            .row_mut(last)
            .copy_from_slice(high_result.row(high_rows - 1));
        return Ok(output);
    }
    for column in 0..columns {
        high_result.set(
            0,
            column,
            high.at(0, column).wrapping_add(low_even.at(0, column)),
        );
    }
    for row in 1..high_rows {
        for column in 0..columns {
            let value = floor_shift(
                low_even
                    .at(row - 1, column)
                    .wrapping_add(low_even.at(row, column)),
                1,
            );
            high_result.set(row, column, high.at(row, column).wrapping_add(value));
        }
    }
    let mut output = Plane::new(low_rows * 2, columns)?;
    for row in 0..low_rows {
        output
            .row_mut(2 * row)
            .copy_from_slice(high_result.row(row));
        output
            .row_mut(2 * row + 1)
            .copy_from_slice(low_even.row(row));
    }
    Ok(output)
}

fn synthesize_level(low: &Plane, s0: &Plane, s1: &Plane, s2: &Plane) -> Result<Plane, CodecError> {
    let (rows, columns) = (low.rows, low.columns);
    require(
        s0.columns == columns && s1.columns == columns && s2.columns == columns,
        "subband widths are invalid",
    )?;
    require(
        s0.rows > rows && s1.rows > rows && s2.rows > rows && rows > 0,
        "subband rows are invalid",
    )?;
    let mut low_high = Plane::new(rows, columns)?;
    let mut high_high = Plane::new(rows, columns)?;
    for row in 0..rows - 1 {
        low_high.row_mut(row).copy_from_slice(s1.row(row + 1));
        high_high.row_mut(row).copy_from_slice(s2.row(row + 1));
    }
    low_high.row_mut(rows - 1).copy_from_slice(s0.row(rows));
    high_high.row_mut(rows - 1).copy_from_slice(s1.row(rows));
    let low = inverse_vertical(low, &low_high)?;
    let high = inverse_vertical(&s0.first_rows(rows)?, &high_high)?;
    inverse_horizontal(&low, &high)
}

fn synthesize_strided(
    low: &Plane,
    s0: &Plane,
    s1: &Plane,
    s2: &Plane,
    edge: usize,
    odd: bool,
) -> Result<Plane, CodecError> {
    if edge == 0 {
        return synthesize_level(low, s0, s1, s2);
    }
    let (rows, columns) = (low.rows, low.columns);
    require(
        s0.columns == columns && s1.columns == columns && s2.columns == columns,
        "subband widths are invalid",
    )?;
    require(
        rows >= 2 * edge && s0.rows >= rows + 2 * edge && s1.rows >= rows + edge && s2.rows >= rows,
        "subband rows are invalid",
    )?;
    let mut high_low = Plane::new(rows, columns)?;
    let mut low_high = Plane::new(rows, columns)?;
    let mut high_high = Plane::new(rows, columns)?;
    for row in 0..edge {
        high_low.row_mut(row).copy_from_slice(s0.row(row));
        low_high.row_mut(row).copy_from_slice(s0.row(row + edge));
        for column in 0..columns {
            high_high.set(row, column, edge_detail(s1.at(row, column), odd));
        }
    }
    for row in edge..rows - edge {
        high_low.row_mut(row).copy_from_slice(s0.row(row + edge));
        low_high.row_mut(row).copy_from_slice(s1.row(row + edge));
        high_high.row_mut(row).copy_from_slice(s2.row(row + edge));
    }
    for row in rows - edge..rows {
        let tail = row - (rows - edge);
        high_low.row_mut(row).copy_from_slice(s0.row(rows + tail));
        low_high
            .row_mut(row)
            .copy_from_slice(s0.row(rows + edge + tail));
        for column in 0..columns {
            high_high.set(row, column, edge_detail(s1.at(rows + tail, column), odd));
        }
    }
    let low = inverse_vertical(low, &low_high)?;
    let high = inverse_vertical(&high_low, &high_high)?;
    inverse_horizontal(&low, &high)
}

fn synthesize_guard_one(
    low: &Plane,
    s0: &Plane,
    s1: &Plane,
    s2: &Plane,
) -> Result<Plane, CodecError> {
    let (rows, columns) = (low.rows, low.columns);
    require(
        s0.columns == columns && s1.columns == columns && s2.columns == columns,
        "subband widths are invalid",
    )?;
    require(
        s0.rows >= rows + 2 && s1.rows >= rows + 2 && s2.rows >= rows + 2,
        "subband rows are invalid",
    )?;
    let mut low_high = Plane::new(rows + 1, columns)?;
    let mut high_high = Plane::new(rows + 1, columns)?;
    let mut high_low = Plane::new(rows, columns)?;
    for row in 0..rows {
        high_low.row_mut(row).copy_from_slice(s0.row(row + 1));
        low_high.row_mut(row).copy_from_slice(s1.row(row + 1));
        high_high.row_mut(row).copy_from_slice(s2.row(row + 1));
    }
    low_high.row_mut(rows).copy_from_slice(s0.row(rows + 1));
    high_high.row_mut(rows).copy_from_slice(s1.row(rows + 1));
    let low = inverse_vertical_high_first(low, &low_high)?;
    let high = inverse_vertical_high_first(&high_low, &high_high)?;
    inverse_horizontal(&low, &high)
}

fn synthesize_guard_two(
    low: &Plane,
    s0: &Plane,
    s1: &Plane,
    s2: &Plane,
) -> Result<Plane, CodecError> {
    let (rows, columns) = (low.rows, low.columns);
    require(
        s0.columns == columns && s1.columns == columns && s2.columns == columns,
        "subband widths are invalid",
    )?;
    require(
        rows > 0 && s0.rows >= rows + 3 && s1.rows > rows && s2.rows > rows,
        "subband rows are invalid",
    )?;
    let mut high_low = Plane::new(rows, columns)?;
    let mut low_high = Plane::new(rows, columns)?;
    let mut high_high = Plane::new(rows, columns)?;
    low_high.row_mut(0).copy_from_slice(s0.row(0));
    for column in 0..columns {
        high_high.set(0, column, edge_detail(s0.at(1, column), false));
    }
    for row in 0..rows {
        high_low.row_mut(row).copy_from_slice(s0.row(row + 2));
    }
    for row in 1..rows {
        low_high.row_mut(row).copy_from_slice(s1.row(row + 1));
        high_high.row_mut(row).copy_from_slice(s2.row(row + 1));
    }
    let low = inverse_vertical_high_first(low, &low_high)?;
    let high = inverse_vertical_high_first(&high_low, &high_high)?;
    inverse_horizontal(&low, &high)
}

fn synthesize_guard_three(
    low: &Plane,
    s0: &Plane,
    s1: &Plane,
    s2: &Plane,
    odd: bool,
) -> Result<Plane, CodecError> {
    let (rows, columns) = (low.rows, low.columns);
    require(
        s0.columns == columns && s1.columns == columns && s2.columns == columns,
        "subband widths are invalid",
    )?;
    require(
        rows >= 2 && s0.rows >= rows + 5 && s1.rows >= rows + 2 && s2.rows >= rows + 2,
        "subband rows are invalid",
    )?;
    let mut high_low = Plane::new(rows, columns)?;
    let mut low_high = Plane::new(rows, columns)?;
    let mut high_high = Plane::new(rows, columns)?;
    high_low.row_mut(0).copy_from_slice(s0.row(0));
    low_high.row_mut(0).copy_from_slice(s0.row(1));
    low_high.row_mut(rows - 1).copy_from_slice(s0.row(rows + 3));
    for column in 0..columns {
        high_high.set(0, column, edge_detail(s0.at(2, column), odd));
        high_high.set(rows - 1, column, edge_detail(s0.at(rows + 4, column), odd));
    }
    for row in 1..rows {
        high_low.row_mut(row).copy_from_slice(s0.row(row + 3));
    }
    for row in 1..rows - 1 {
        low_high.row_mut(row).copy_from_slice(s1.row(row + 3));
        high_high.row_mut(row).copy_from_slice(s2.row(row + 3));
    }
    let low = inverse_vertical(low, &low_high)?;
    let high = inverse_vertical(&high_low, &high_high)?;
    inverse_horizontal(&low, &high)
}

fn final_green(low: &Plane, detail: &Plane, top_rows: usize) -> Result<Plane, CodecError> {
    let (rows, columns) = (low.rows, low.columns);
    require(
        top_rows <= 8
            && detail.columns == columns
            && detail.rows >= 8 + rows.saturating_sub(top_rows),
        "green detail size is invalid",
    )?;
    if rows == 0 || columns == 0 {
        return Plane::new(rows, columns * 2);
    }
    let selected_row = |row: usize| {
        detail.row(if row < top_rows {
            row
        } else {
            row + 8 - top_rows
        })
    };
    let mut odd = Plane::new(rows, columns)?;
    odd.samples
        .par_chunks_mut(columns)
        .enumerate()
        .for_each(|(row, destination)| {
            let current = selected_row(row);
            let previous = selected_row(row.saturating_sub(1));
            let low = low.row(row);
            for column in 0..columns {
                let prediction = if column + 1 < columns {
                    floor_shift(
                        current[column + 1]
                            .wrapping_add(previous[column])
                            .wrapping_add(current[column])
                            .wrapping_add(previous[column + 1]),
                        2,
                    )
                } else {
                    floor_shift(
                        previous[column]
                            .wrapping_add(current[column])
                            .wrapping_mul(2),
                        2,
                    )
                };
                destination[column] =
                    ((2 * i64::from(low[column]) - i64::from(prediction)) >> 1) as i32;
            }
        });
    let mut output = Plane::new(rows, columns * 2)?;
    output
        .samples
        .par_chunks_mut(2 * columns)
        .enumerate()
        .for_each(|(row, destination)| {
            let current_odd = odd.row(row);
            let next_odd = odd.row(if row + 1 < rows { row + 1 } else { row });
            let selected = selected_row(row);
            let first = (((i64::from(current_odd[0]) + i64::from(next_odd[0])) * 2) >> 2) as i32;
            destination[0] = selected[0].wrapping_add(first);
            destination[1] = current_odd[0];
            for column in 1..columns {
                let sum = current_odd[column - 1]
                    .wrapping_add(next_odd[column - 1])
                    .wrapping_add(next_odd[column])
                    .wrapping_add(current_odd[column]);
                destination[2 * column] = selected[column].wrapping_add(floor_shift(sum, 2));
                destination[2 * column + 1] = current_odd[column];
            }
        });
    Ok(output)
}

struct DecodedTile {
    green: Plane,
    red: Plane,
    blue: Plane,
}

fn decode_stream(stream: &[u8]) -> Result<DecodedTile, CodecError> {
    let (width, half_height) =
        parse_stream_header(stream).ok_or_else(|| error("invalid stream header"))?;
    let directory = parse_directory(stream)?;
    let coded_height = half_height * 2;
    let padded_height = coded_height.div_ceil(16) * 16;
    let guarded = coded_height != padded_height;
    let low_rows = padded_height / 16;
    let low_start = usize::from(guarded);
    let plane = |index| -> Result<Plane, CodecError> {
        let band_zero = decode_packet_planes(stream, &directory, width, coded_height, 0, index)?;
        let mut plane = integrate(&band_zero[0], low_rows, low_start)?;
        for group in 1..=3 {
            let edge = [0, 0, 1, 2][group];
            let odd = group == 3 && index != 0;
            let detail =
                decode_packet_planes(stream, &directory, width, coded_height, group, index)?;
            require(
                detail.len() == 3,
                "detail packet component count is invalid",
            )?;
            plane = if guarded {
                match group {
                    1 => synthesize_guard_one(&plane, &detail[0], &detail[1], &detail[2])?,
                    2 => synthesize_guard_two(&plane, &detail[0], &detail[1], &detail[2])?,
                    _ => synthesize_guard_three(&plane, &detail[0], &detail[1], &detail[2], odd)?,
                }
            } else {
                synthesize_strided(&plane, &detail[0], &detail[1], &detail[2], edge, odd)?
            };
        }
        Ok(plane)
    };
    let mut planes = [0, 1, 2]
        .into_par_iter()
        .map(plane)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter();
    let green = planes
        .next()
        .ok_or_else(|| error("green plane is missing"))?;
    let red = planes.next().ok_or_else(|| error("red plane is missing"))?;
    let blue = planes
        .next()
        .ok_or_else(|| error("blue plane is missing"))?;
    let green_detail = decode_packet_planes(stream, &directory, width, coded_height, 4, 0)?;
    let green = final_green(&green, &green_detail[0], if guarded { 2 } else { 4 })?;
    Ok(DecodedTile { green, red, blue })
}

#[inline]
fn clamp_code(value: i32) -> i32 { value.saturating_add(BIAS).clamp(0, 4095) - BIAS }

#[inline]
fn map_sample(value: i32) -> u16 { LLVC3_CURVE[value.saturating_add(BIAS).clamp(0, 4095) as usize] }

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("Sony ARW6 requires integer samples"));
    }
    require(
        request.width > 0
            && request.height > 0
            && request.width.is_multiple_of(16)
            && request.height.is_multiple_of(2),
        "frame size is invalid",
    )?;
    require(
        request.chunk_width == request.width && request.chunk_height == request.height,
        "Sony ARW6 requires one whole-image chunk",
    )?;
    require(
        request.chunks.len() == 1,
        "Sony ARW6 chunk count is invalid",
    )?;
    let strip = chunk(request.source, request.chunks[0])?;
    let sample_count = request
        .width
        .checked_mul(request.height)
        .ok_or_else(|| error("sample count overflows"))?;
    let mut output = vec![0_u16; sample_count];
    let streams = locate_streams(strip, request.width, request.height);
    require(!streams.is_empty(), "no tile streams were found")?;
    let tiles = streams
        .par_iter()
        .map(|stream| decode_stream(&strip[stream.offset..stream.offset + stream.length]))
        .collect::<Vec<_>>();
    for (stream, tile) in streams.into_iter().zip(tiles) {
        let tile = tile?;
        let half_height = stream.height / 2;
        let half_width = stream.width / 2;
        require(
            tile.green.rows >= half_height
                && tile.green.columns == stream.width
                && tile.red.rows >= half_height
                && tile.red.columns == half_width
                && tile.blue.rows >= half_height
                && tile.blue.columns == half_width,
            "tile plane sizes are invalid",
        )?;
        let rows =
            &mut output[stream.y * request.width..(stream.y + stream.height) * request.width];
        rows.par_chunks_mut(2 * request.width)
            .enumerate()
            .for_each(|(row_pair, pair)| {
                let (first_row, second_row) = pair.split_at_mut(request.width);
                let first_row = &mut first_row[stream.x..stream.x + stream.width];
                let second_row = &mut second_row[stream.x..stream.x + stream.width];
                let green = tile.green.row(row_pair);
                let red = tile.red.row(row_pair);
                let blue = tile.blue.row(row_pair);
                for column in 0..half_width {
                    let green0 = clamp_code(green[2 * column]);
                    let green1 = clamp_code(green[2 * column + 1]);
                    let average = ((i64::from(green0) + i64::from(green1)) >> 1) as i32;
                    first_row[2 * column] =
                        map_sample(average.wrapping_add(red[column].wrapping_mul(2)));
                    first_row[2 * column + 1] = map_sample(green[2 * column + 1]);
                    second_row[2 * column] = map_sample(green[2 * column]);
                    second_row[2 * column + 1] =
                        map_sample(average.wrapping_add(blue[column].wrapping_mul(2)));
                }
            });
    }
    buffer_u16(request.width, request.height, output)
}

#[cfg(test)]
mod tests {
    use super::{Plane, edge_detail, floor_shift, inverse_horizontal, sign_extend_16};

    #[test]
    fn signed_floor_shifts_match_floor_division() {
        for value in -9..=9 {
            assert_eq!(floor_shift(value, 1), (value as f64 / 2.0).floor() as i32);
            assert_eq!(floor_shift(value, 2), (value as f64 / 4.0).floor() as i32);
        }
    }

    #[test]
    fn edge_details_follow_even_and_odd_boundary_modes() {
        assert_eq!(edge_detail(3, true), 7);
        assert_eq!(edge_detail(3, false), 6);
        assert_eq!(edge_detail(-4, false), -9);
        assert_eq!(edge_detail(-4, true), -8);
        assert_eq!(sign_extend_16(0xffff), -1);
    }

    #[test]
    fn inverse_horizontal_restores_a_forward_five_three_signal() {
        let signal = [10, 13, 7, 2, 8, 30, 31, 29, 5, 6];
        let count = signal.len() / 2;
        let mut high = vec![0_i32; count];
        let mut low = vec![0_i32; count];
        for index in 0..count {
            let right = if 2 * index + 2 < signal.len() {
                signal[2 * index + 2]
            } else {
                signal[2 * index]
            };
            high[index] = signal[2 * index + 1] - floor_shift(signal[2 * index] + right, 1);
        }
        for index in 0..count {
            let previous = high[index.saturating_sub(1)];
            low[index] = signal[2 * index] + floor_shift(previous + high[index] + 2, 2);
        }
        let low = Plane {
            rows: 1,
            columns: count,
            samples: low,
        };
        let high = Plane {
            rows: 1,
            columns: count,
            samples: high,
        };

        assert_eq!(inverse_horizontal(&low, &high).unwrap().samples, signal);
    }
}
