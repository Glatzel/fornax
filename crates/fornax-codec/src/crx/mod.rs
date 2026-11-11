//! Canon CRX lossless CFA decoding.

use rayon::prelude::*;

use crate::error::{CodecError, invalid};
use crate::huffman::BitReader;
use crate::{CanonCrxHeader, DecodeRequest, SampleBuffer};

const TILE_MARKER: u16 = 0xff01;
const PLANE_MARKER: u16 = 0xff02;
const SUBBAND_MARKER: u16 = 0xff03;
const PLANE_COUNT: usize = 4;
const PREDICT_K_MAX: u32 = 15;
const PREDICT_K_ESCAPE: u32 = 41;
const PREDICT_K_ESCAPE_BITS: u8 = 21;
const QP_STEP: [u32; 6] = [0x28, 0x2d, 0x33, 0x39, 0x40, 0x48];

const RUN_J: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13,
    14, 15,
];

/// A conventional 2x2 RGB Bayer arrangement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonCrxCfa {
    /// Red, green, green, blue.
    Rggb,
    /// Green, red, blue, green.
    Grbg,
    /// Green, blue, red, green.
    Gbrg,
    /// Blue, green, green, red.
    Bggr,
}

impl CanonCrxCfa {
    /// Maps a CMP1 CFA-layout nibble to its Bayer arrangement.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::Unsupported`] for non-Bayer layout values.
    pub fn from_cmp1_layout(layout: u8) -> Result<Self, CodecError> {
        match layout {
            0 => Ok(Self::Rggb),
            1 => Ok(Self::Grbg),
            2 => Ok(Self::Gbrg),
            3 => Ok(Self::Bggr),
            _ => Err(CodecError::Unsupported(
                "Canon CRX CFA layout is not conventional 2x2 RGB Bayer".into(),
            )),
        }
    }

    /// Returns the pattern in row-major RGB-channel order.
    pub const fn pattern(self) -> [u8; 4] {
        match self {
            Self::Rggb => [0, 1, 1, 2],
            Self::Grbg => [1, 0, 2, 1],
            Self::Gbrg => [1, 2, 0, 1],
            Self::Bggr => [2, 1, 1, 0],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Band {
    data_offset: usize,
    data_size: usize,
    padded_size: usize,
}

#[derive(Clone, Copy, Debug)]
struct Plane {
    band: Band,
}

#[derive(Clone, Debug)]
struct Tile {
    row: usize,
    col: usize,
    width: usize,
    height: usize,
    planes: [Plane; PLANE_COUNT],
}

/// Decodes a validated lossless CRX sample into one U16 mosaic.
pub fn decode(
    request: &DecodeRequest<'_>,
    header: &CanonCrxHeader,
    _cfa: CanonCrxCfa,
) -> Result<SampleBuffer, CodecError> {
    if request.chunks.len() != 1 {
        return Err(invalid("Canon CRX requires exactly one raw sample"));
    }
    if request.width != usize::try_from(header.mosaic_width()).unwrap_or(usize::MAX)
        || request.height != usize::try_from(header.mosaic_height()).unwrap_or(usize::MAX)
    {
        return Err(invalid("Canon CRX request geometry disagrees with CMP1"));
    }
    let chunk = request.chunks[0];
    let end = chunk
        .offset
        .checked_add(chunk.byte_count)
        .ok_or_else(|| invalid("Canon CRX sample range overflow"))?;
    let sample = request
        .source
        .get(chunk.offset..end)
        .ok_or_else(|| invalid("Canon CRX sample is outside the source"))?;
    let header_size = usize::try_from(header.mdat_header_size())
        .map_err(|_| invalid("Canon CRX MDAT header size exceeds usize"))?;
    if sample.len() < header_size {
        return Err(invalid("Canon CRX sample is shorter than its MDAT header"));
    }
    let (header_bytes, data) = sample.split_at(header_size);
    let mut tiles = parse_tiles(header_bytes, header)?;
    assign_tile_dimensions(&mut tiles, header)?;
    validate_payload(tiles.iter().flat_map(|tile| tile.planes.iter()), data.len())?;

    let mut tasks = Vec::with_capacity(tiles.len() * PLANE_COUNT);
    for (tile_index, tile) in tiles.iter().enumerate() {
        for (plane_index, plane) in tile.planes.iter().enumerate() {
            tasks.push((tile_index, plane_index, *plane));
        }
    }
    let decoded: Result<Vec<(usize, usize, Vec<u16>)>, CodecError> = tasks
        .par_iter()
        .map(|(tile_index, plane_index, plane)| {
            let band = plane
                .band
                .data_offset
                .checked_add(plane.band.data_size)
                .and_then(|end| data.get(plane.band.data_offset..end))
                .ok_or_else(|| invalid("Canon CRX band exceeds the sample"))?;
            let tile = &tiles[*tile_index];
            decode_plane(tile.width / 2, tile.height / 2, band, header)
                .map(|samples| (*tile_index, *plane_index, samples))
        })
        .collect();
    let decoded = decoded?;

    let pixel_count =
        usize::try_from(u64::from(header.mosaic_width()) * u64::from(header.mosaic_height()))
            .map_err(|_| invalid("Canon CRX mosaic dimensions exceed usize"))?;
    let mut mosaic = vec![0_u16; pixel_count];
    for (tile_index, plane_index, plane) in decoded {
        let tile = &tiles[tile_index];
        integrate_plane(&mut mosaic, tile, plane_index, &plane, header)?;
    }
    SampleBuffer::from_u16(header.mosaic_width(), header.mosaic_height(), mosaic)
        .map_err(Into::into)
}

fn parse_tiles(bytes: &[u8], header: &CanonCrxHeader) -> Result<Vec<Tile>, CodecError> {
    let mut reader = HeaderReader::new(bytes);
    let mut tiles = Vec::new();
    let mut tile_data_offset = 0_usize;
    let mut marker = reader.u16()?;
    loop {
        match marker {
            0 => break,
            TILE_MARKER => {
                let header_size = reader.u16()?;
                if header_size != 8 {
                    return Err(CodecError::Unsupported(
                        "Canon CRX lossless tile QP headers are unsupported".into(),
                    ));
                }
                let tile_size = usize::try_from(reader.u32()?)
                    .map_err(|_| invalid("Canon CRX tile size exceeds usize"))?;
                let flags = reader.u32()?;
                if flags & 0xffff != 0 {
                    return Err(invalid("Canon CRX tile tail marker is invalid"));
                }
                let mut planes = Vec::with_capacity(PLANE_COUNT);
                let mut plane_offset = 0_usize;
                for _ in 0..PLANE_COUNT {
                    if reader.u16()? != PLANE_MARKER {
                        return Err(invalid("Canon CRX plane marker is missing"));
                    }
                    if reader.u16()? != 8 {
                        return Err(invalid("Canon CRX plane header size is invalid"));
                    }
                    let plane_size = usize::try_from(reader.u32()?)
                        .map_err(|_| invalid("Canon CRX plane size exceeds usize"))?;
                    let flags = reader.u32()?;
                    if flags & 0x00ff_ffff != 0 {
                        return Err(invalid("Canon CRX plane flags are malformed"));
                    }
                    if flags & 0x0800_0000 == 0 {
                        return Err(CodecError::Unsupported(
                            "Canon CRX lossless planes must use MED prediction".into(),
                        ));
                    }
                    let rounded_bits = (flags >> 25) & 0x3;
                    if rounded_bits != 0 {
                        return Err(CodecError::Unsupported(
                            "Canon CRX rounded lossless planes are unvalidated".into(),
                        ));
                    }
                    if reader.u16()? != SUBBAND_MARKER {
                        return Err(invalid("Canon CRX subband marker is missing"));
                    }
                    if reader.u16()? != 8 {
                        return Err(invalid("Canon CRX subband header size is invalid"));
                    }
                    let padded_size = usize::try_from(reader.u32()?)
                        .map_err(|_| invalid("Canon CRX subband size exceeds usize"))?;
                    let band_flags = reader.u32()?;
                    if band_flags & 0x0800_0000 != 0 {
                        return Err(CodecError::Unsupported(
                            "Canon CRX partial subband decoding is unsupported".into(),
                        ));
                    }
                    let q_param = u8::try_from((band_flags >> 19) & 0xff)
                        .map_err(|_| invalid("Canon CRX QP is out of range"))?;
                    let unused = usize::try_from(band_flags & 0x7ffff)
                        .map_err(|_| invalid("Canon CRX unused byte count exceeds usize"))?;
                    if padded_size == 0 || padded_size % 8 != 0 || unused >= padded_size {
                        return Err(invalid("Canon CRX subband padding is invalid"));
                    }
                    let data_size = padded_size - unused;
                    if plane_size != padded_size {
                        return Err(CodecError::Unsupported(
                            "Canon CRX lossless plane must contain exactly one subband".into(),
                        ));
                    }
                    validate_qp(q_param)?;
                    let data_offset = tile_data_offset
                        .checked_add(plane_offset)
                        .ok_or_else(|| invalid("Canon CRX plane offset overflow"))?;
                    planes.push(Plane {
                        band: Band {
                            data_offset,
                            data_size,
                            padded_size,
                        },
                    });
                    plane_offset = plane_offset
                        .checked_add(plane_size)
                        .ok_or_else(|| invalid("Canon CRX plane size overflow"))?;
                }
                if plane_offset != tile_size {
                    return Err(invalid("Canon CRX tile size disagrees with its planes"));
                }
                let planes: [Plane; PLANE_COUNT] = planes
                    .try_into()
                    .map_err(|_| invalid("Canon CRX tile must contain four planes"))?;
                let expected_tiles = expected_tile_count(header)?;
                if tiles.len() >= expected_tiles {
                    return Err(invalid("Canon CRX header contains excess tiles"));
                }
                tiles.push(Tile {
                    row: 0,
                    col: 0,
                    width: 0,
                    height: 0,
                    planes,
                });
                tile_data_offset = tile_data_offset
                    .checked_add(tile_size)
                    .ok_or_else(|| invalid("Canon CRX tile data offset overflow"))?;
                marker = reader.u16()?;
            }
            value => {
                return Err(invalid(format!(
                    "Canon CRX header has unexpected marker 0x{value:04x}"
                )));
            }
        }
    }
    if tiles.len() != expected_tile_count(header)? {
        return Err(invalid("Canon CRX header tile count disagrees with CMP1"));
    }
    Ok(tiles)
}

fn validate_qp(q_param: u8) -> Result<(), CodecError> {
    let q = u32::from(q_param);
    let scale = if q / 6 >= 6 {
        QP_STEP
            .get(usize::try_from(q % 6).unwrap_or(0))
            .copied()
            .unwrap_or(1)
            .checked_shl(q / 6 + 26)
            .unwrap_or(u32::MAX)
    } else {
        QP_STEP
            .get(usize::try_from(q % 6).unwrap_or(0))
            .copied()
            .unwrap_or(1)
            >> (6 - q / 6)
    };
    if scale != 1 {
        return Err(CodecError::Unsupported(
            "Canon CRX lossless Q-step is not identity".into(),
        ));
    }
    Ok(())
}

fn expected_tile_count(header: &CanonCrxHeader) -> Result<usize, CodecError> {
    let cols = header.mosaic_width().div_ceil(header.tile_width());
    let rows = header.mosaic_height().div_ceil(header.tile_height());
    let count = u64::from(cols) * u64::from(rows);
    usize::try_from(count).map_err(|_| invalid("Canon CRX tile count exceeds usize"))
}

fn assign_tile_dimensions(tiles: &mut [Tile], header: &CanonCrxHeader) -> Result<(), CodecError> {
    let cols = usize::try_from(header.mosaic_width().div_ceil(header.tile_width()))
        .map_err(|_| invalid("Canon CRX tile column count exceeds usize"))?;
    for (index, tile) in tiles.iter_mut().enumerate() {
        let row = index / cols;
        let col = index % cols;
        let x = u64::from(header.tile_width()) * u64::try_from(col).unwrap_or(u64::MAX);
        let y = u64::from(header.tile_height()) * u64::try_from(row).unwrap_or(u64::MAX);
        let width = u64::from(header.mosaic_width())
            .saturating_sub(x)
            .min(u64::from(header.tile_width()));
        let height = u64::from(header.mosaic_height())
            .saturating_sub(y)
            .min(u64::from(header.tile_height()));
        tile.width =
            usize::try_from(width).map_err(|_| invalid("Canon CRX tile width exceeds usize"))?;
        tile.height =
            usize::try_from(height).map_err(|_| invalid("Canon CRX tile height exceeds usize"))?;
        tile.row = row;
        tile.col = col;
        if tile.width == 0 || tile.height == 0 || tile.width % 2 != 0 || tile.height % 2 != 0 {
            return Err(CodecError::Unsupported(
                "Canon CRX tile dimensions are not conventional 2x2 Bayer".into(),
            ));
        }
    }
    Ok(())
}

fn validate_payload<'a>(
    planes: impl Iterator<Item = &'a Plane>,
    data_len: usize,
) -> Result<(), CodecError> {
    let mut compressed = 0_usize;
    for plane in planes {
        compressed = compressed
            .checked_add(plane.band.data_size)
            .ok_or_else(|| invalid("Canon CRX compressed size overflow"))?;
        let padded_end = plane
            .band
            .data_offset
            .checked_add(plane.band.padded_size)
            .ok_or_else(|| invalid("Canon CRX band range overflow"))?;
        if padded_end > data_len {
            return Err(invalid("Canon CRX band exceeds sample payload"));
        }
    }
    if compressed == 0 || compressed > data_len {
        return Err(invalid("Canon CRX payload has implausible compressed size"));
    }
    Ok(())
}

fn decode_plane(
    width: usize,
    height: usize,
    encoded: &[u8],
    header: &CanonCrxHeader,
) -> Result<Vec<u16>, CodecError> {
    let mut decoder = PlaneDecoder::new(width, height, encoded);
    let median = 1_i32
        .checked_shl(u32::from(header.median_precision()) - 1)
        .ok_or_else(|| invalid("Canon CRX median precision is invalid"))?;
    let max_val = 1_i32
        .checked_shl(u32::from(header.median_precision()))
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| invalid("Canon CRX sample precision is invalid"))?;
    let mut output = vec![0_u16; width * height];
    for row in 0..height {
        decoder.decode_line()?;
        let line = decoder.current();
        let start = row * width;
        for (target, value) in output[start..start + width].iter_mut().zip(line) {
            *target = u16::try_from(
                value
                    .checked_add(median)
                    .ok_or_else(|| invalid("Canon CRX sample prediction overflow"))?
                    .clamp(0, max_val),
            )
            .map_err(|_| invalid("Canon CRX sample exceeds u16"))?;
        }
    }
    Ok(output)
}

fn integrate_plane(
    mosaic: &mut [u16],
    tile: &Tile,
    plane_index: usize,
    plane: &[u16],
    header: &CanonCrxHeader,
) -> Result<(), CodecError> {
    let tile_x = tile
        .col
        .checked_mul(usize::try_from(header.tile_width()).unwrap_or(usize::MAX))
        .ok_or_else(|| invalid("Canon CRX tile X offset overflow"))?;
    let tile_y = tile
        .row
        .checked_mul(usize::try_from(header.tile_height()).unwrap_or(usize::MAX))
        .ok_or_else(|| invalid("Canon CRX tile Y offset overflow"))?;
    let plane_width = tile.width / 2;
    let plane_height = tile.height / 2;
    if plane.len() != plane_width * plane_height {
        return Err(invalid(
            "Canon CRX decoded plane dimensions are inconsistent",
        ));
    }
    let row_shift = plane_index / 2;
    let col_shift = plane_index % 2;
    let mosaic_width = usize::try_from(header.mosaic_width())
        .map_err(|_| invalid("Canon CRX mosaic width exceeds usize"))?;
    let mosaic_height = usize::try_from(header.mosaic_height())
        .map_err(|_| invalid("Canon CRX mosaic height exceeds usize"))?;
    for row in 0..plane_height {
        let y = tile_y
            .checked_add(row * 2 + row_shift)
            .ok_or_else(|| invalid("Canon CRX output row overflow"))?;
        if y >= mosaic_height {
            return Err(invalid("Canon CRX output row exceeds mosaic"));
        }
        for col in 0..plane_width {
            let x = tile_x
                .checked_add(col * 2 + col_shift)
                .ok_or_else(|| invalid("Canon CRX output column overflow"))?;
            if x >= mosaic_width {
                return Err(invalid("Canon CRX output column exceeds mosaic"));
            }
            let target = y
                .checked_mul(mosaic_width)
                .and_then(|offset| offset.checked_add(x))
                .ok_or_else(|| invalid("Canon CRX output offset overflow"))?;
            mosaic[target] = plane[row * plane_width + col];
        }
    }
    Ok(())
}

struct PlaneDecoder<'a> {
    width: usize,
    height: usize,
    current_row: usize,
    lines: [Vec<i32>; 2],
    line_pos: usize,
    s_param: usize,
    k: u32,
    bits: BitReader<'a>,
}

impl<'a> PlaneDecoder<'a> {
    fn new(width: usize, height: usize, encoded: &'a [u8]) -> Self {
        let line_len = width.checked_add(2).unwrap_or(2);
        Self {
            width,
            height,
            current_row: 0,
            lines: [vec![0; line_len], vec![0; line_len]],
            line_pos: 1,
            s_param: 0,
            k: 0,
            bits: BitReader::new(encoded),
        }
    }

    fn current(&self) -> &[i32] { &self.lines[1][1..=self.width] }

    fn decode_line(&mut self) -> Result<(), CodecError> {
        if self.current_row >= self.height {
            return Err(invalid("Canon CRX plane has excess rows"));
        }
        self.line_pos = 1;
        if self.current_row == 0 {
            self.s_param = 0;
            self.k = 0;
            self.decode_top_line()?;
        } else {
            self.lines.swap(0, 1);
            self.decode_nontop_line()?;
        }
        self.current_row += 1;
        Ok(())
    }

    fn decode_top_line(&mut self) -> Result<(), CodecError> {
        let mut remaining = self.width;
        self.lines[1][0] = 0;
        while remaining > 1 {
            let a = self.a();
            if a != 0 {
                self.lines[1][self.line_pos] = a;
            } else {
                if self.bits.read_bits(1)? == 1 {
                    let count = self.run_count(remaining)?;
                    if count > remaining {
                        return Err(invalid("Canon CRX run exceeds line width"));
                    }
                    for _ in 0..count {
                        self.lines[1][self.line_pos] = a;
                        self.line_pos += 1;
                    }
                    remaining -= count;
                    if remaining == 0 {
                        break;
                    }
                }
                self.lines[1][self.line_pos] = 0;
            }
            let code = self.rice(true, PREDICT_K_MAX)?;
            self.lines[1][self.line_pos] = self.lines[1][self.line_pos]
                .checked_add(signed(code)?)
                .ok_or_else(|| invalid("Canon CRX line prediction overflow"))?;
            self.line_pos += 1;
            remaining -= 1;
        }
        if remaining == 1 {
            let code = self.rice(true, PREDICT_K_MAX)?;
            self.lines[1][self.line_pos] = self
                .a()
                .checked_add(signed(code)?)
                .ok_or_else(|| invalid("Canon CRX line prediction overflow"))?;
            self.line_pos += 1;
        }
        self.finish_line()?;
        Ok(())
    }

    fn decode_nontop_line(&mut self) -> Result<(), CodecError> {
        let mut remaining = self.width;
        self.lines[1][0] = self.b();
        while remaining > 1 {
            let (a, b, c, d) = (self.a(), self.b(), self.c(), self.d());
            let prediction = if a == b && a == d {
                if self.bits.read_bits(1)? == 1 {
                    let count = self.run_count(remaining)?;
                    if count > remaining {
                        return Err(invalid("Canon CRX run exceeds line width"));
                    }
                    for _ in 0..count {
                        self.lines[1][self.line_pos] = a;
                        self.line_pos += 1;
                    }
                    remaining -= count;
                }
                if remaining > 0 { self.b() } else { 0 }
            } else {
                med(a, b, c)
            };
            if remaining > 0 {
                let code = self.rice(false, PREDICT_K_MAX)?;
                self.lines[1][self.line_pos] = prediction
                    .checked_add(signed(code)?)
                    .ok_or_else(|| invalid("Canon CRX line prediction overflow"))?;
                if remaining > 1 {
                    let delta = i64::from(self.d()) - i64::from(self.b());
                    let magnitude = delta
                        .checked_abs()
                        .and_then(|value| value.checked_mul(2))
                        .ok_or_else(|| invalid("Canon CRX K predictor overflow"))?;
                    let magnitude = u32::try_from(magnitude)
                        .map_err(|_| invalid("Canon CRX K predictor exceeds u32"))?;
                    self.predict_k(
                        code.checked_add(magnitude)
                            .ok_or_else(|| invalid("Canon CRX K predictor overflow"))?
                            >> 1,
                        PREDICT_K_MAX,
                    );
                } else {
                    self.predict_k(code, PREDICT_K_MAX);
                }
                self.line_pos += 1;
            }
            remaining = remaining.saturating_sub(1);
        }
        if remaining == 1 {
            let prediction = med(self.a(), self.b(), self.c());
            let code = self.rice(true, PREDICT_K_MAX)?;
            self.lines[1][self.line_pos] = prediction
                .checked_add(signed(code)?)
                .ok_or_else(|| invalid("Canon CRX line prediction overflow"))?;
            self.line_pos += 1;
        }
        self.finish_line()?;
        Ok(())
    }

    fn finish_line(&mut self) -> Result<(), CodecError> {
        if self.line_pos >= self.lines[1].len() {
            return Err(invalid("Canon CRX decoded line exceeds its buffer"));
        }
        self.lines[1][self.line_pos] = self
            .a()
            .checked_add(1)
            .ok_or_else(|| invalid("Canon CRX line sentinel overflow"))?;
        Ok(())
    }

    fn rice(&mut self, adapt: bool, k_max: u32) -> Result<u32, CodecError> {
        let mut prefix = 0_u32;
        while self.bits.read_bits(1)? == 0 {
            prefix = prefix
                .checked_add(1)
                .ok_or_else(|| invalid("Canon CRX Rice prefix overflow"))?;
            if prefix > PREDICT_K_ESCAPE {
                break;
            }
        }
        let value = if prefix >= PREDICT_K_ESCAPE {
            self.bits.read_bits(PREDICT_K_ESCAPE_BITS)?
        } else if self.k > 0 {
            let remainder = self
                .bits
                .read_bits(u8::try_from(self.k).map_err(|_| invalid("Canon CRX K exceeds u8"))?)?;
            prefix
                .checked_shl(self.k)
                .and_then(|value| value.checked_add(remainder))
                .ok_or_else(|| invalid("Canon CRX Rice value overflow"))?
        } else {
            prefix
        };
        if adapt {
            self.predict_k(value, k_max);
        }
        Ok(value)
    }

    fn predict_k(&mut self, value: u32, k_max: u32) {
        let shifted = value.checked_shr(self.k).unwrap_or(0);
        let mut next = self.k;
        if shifted > 2 {
            next = next.saturating_add(1);
        }
        if shifted > 5 {
            next = next.saturating_add(1);
        }
        let half = 1_u32.checked_shl(self.k).unwrap_or(0) >> 1;
        if value < half {
            next = next.saturating_sub(1);
        }
        self.k = if k_max > 0 { next.min(k_max) } else { next };
    }

    fn run_count(&mut self, remaining: usize) -> Result<usize, CodecError> {
        let mut count = 1_usize;
        while count != remaining && self.bits.read_bits(1)? == 1 {
            let shift = RUN_J[self.s_param];
            count = count.saturating_add(1_usize.checked_shl(shift).unwrap_or(usize::MAX));
            if count > remaining {
                count = remaining;
                break;
            }
            self.s_param = (self.s_param + 1).min(31);
        }
        if count < remaining {
            let bits = RUN_J[self.s_param];
            if bits > 0 {
                let extra = self.bits.read_bits(
                    u8::try_from(bits).map_err(|_| invalid("Canon CRX run bits exceed u8"))?,
                )?;
                count = count
                    .checked_add(usize::try_from(extra).unwrap_or(usize::MAX))
                    .ok_or_else(|| invalid("Canon CRX run count overflow"))?;
            }
            self.s_param = self.s_param.saturating_sub(1);
            if count > remaining {
                return Err(invalid("Canon CRX run exceeds remaining samples"));
            }
        }
        Ok(count)
    }

    fn a(&self) -> i32 { self.lines[1][self.line_pos - 1] }
    fn b(&self) -> i32 { self.lines[0][self.line_pos] }
    fn c(&self) -> i32 { self.lines[0][self.line_pos - 1] }
    fn d(&self) -> i32 { self.lines[0][self.line_pos + 1] }
}

fn signed(code: u32) -> Result<i32, CodecError> {
    let magnitude = code >> 1;
    if magnitude > i32::MAX as u32 {
        return Err(invalid("Canon CRX signed coefficient exceeds i32"));
    }
    let magnitude =
        i32::try_from(magnitude).map_err(|_| invalid("Canon CRX coefficient exceeds i32"))?;
    if code & 1 == 0 {
        Ok(magnitude)
    } else {
        magnitude
            .checked_neg()
            .and_then(|value| value.checked_sub(1))
            .ok_or_else(|| invalid("Canon CRX signed coefficient overflow"))
    }
}

fn med(a: i32, b: i32, c: i32) -> i32 {
    if c >= a.max(b) {
        a.min(b)
    } else if c <= a.min(b) {
        a.max(b)
    } else {
        a.saturating_add(b).saturating_sub(c)
    }
}

struct HeaderReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> HeaderReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn u16(&mut self) -> Result<u16, CodecError> {
        let bytes: [u8; 2] = self
            .bytes(2)?
            .try_into()
            .map_err(|_| invalid("Canon CRX u16 is truncated"))?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        let bytes: [u8; 4] = self
            .bytes(4)?
            .try_into()
            .map_err(|_| invalid("Canon CRX u32 is truncated"))?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn bytes(&mut self, count: usize) -> Result<&'a [u8], CodecError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("Canon CRX header position overflow"))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| invalid("Canon CRX MDAT header is truncated"))?;
        self.position = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rice_escape_and_k_adaptation_are_bounded() {
        let mut bits = vec![false; 63];
        bits[41] = true;
        bits[62] = true;
        let mut escape = vec![0_u8; bits.len().div_ceil(8)];
        for (index, bit) in bits.into_iter().enumerate() {
            if bit {
                escape[index / 8] |= 0x80 >> (index % 8);
            }
        }
        let mut decoder = PlaneDecoder::new(2, 1, &escape);
        assert_eq!(decoder.rice(false, PREDICT_K_MAX).unwrap(), 1);

        let mut decoder = PlaneDecoder::new(2, 1, &[0b0010_0000]);
        assert_eq!(decoder.rice(true, PREDICT_K_MAX).unwrap(), 0b10);
        assert_eq!(decoder.k, 0);
    }

    #[test]
    fn med_prediction_handles_edges() {
        assert_eq!(med(3, 1, 2), 2);
        assert_eq!(med(3, 1, 9), 1);
        assert_eq!(med(3, 1, -2), 3);
        assert_eq!(med(2, 4, 3), 3);
    }

    #[test]
    fn signed_codes_match_crx_sign_mapping() {
        assert_eq!(signed(0).unwrap(), 0);
        assert_eq!(signed(1).unwrap(), -1);
        assert_eq!(signed(2).unwrap(), 1);
        assert_eq!(signed(3).unwrap(), -2);
    }

    #[test]
    fn run_length_saturates_at_the_remaining_line_width() {
        let mut decoder = PlaneDecoder::new(3, 1, &[0b1111_1000]);
        assert_eq!(decoder.run_count(3).unwrap(), 3);
    }

    #[test]
    fn lossless_q_step_must_be_identity() {
        assert!(validate_qp(4).is_ok());
        assert!(validate_qp(0).is_err());
    }
}
