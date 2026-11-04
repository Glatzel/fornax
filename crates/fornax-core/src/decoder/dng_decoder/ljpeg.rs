//! Restart-free lossless JPEG-92 color-component decoding.
//!
//! The DNG-specific tile assembly and metadata decisions are documented in
//! `docs/references/dng-lossless-jpeg.md`.

const MARKER_PREFIX: u8 = 0xff;
const MARKER_SOF3: u8 = 0xc3;
const MARKER_DHT: u8 = 0xc4;
const MARKER_SOI: u8 = 0xd8;
const MARKER_EOI: u8 = 0xd9;
const MARKER_SOS: u8 = 0xda;
const HUFFMAN_LOOKAHEAD_BITS: usize = 16;
const HUFFMAN_LOOKAHEAD_SIZE: usize = 1 << HUFFMAN_LOOKAHEAD_BITS;

#[derive(Clone, Copy)]
struct Component {
    id: u8,
    huffman_table: usize,
}

#[derive(Clone, Copy)]
struct HuffmanEntry {
    symbol: u8,
    bits: u8,
}

impl HuffmanEntry {
    const INVALID: Self = Self { symbol: 0, bits: 0 };
}

struct HuffmanTable {
    lookup: Box<[HuffmanEntry]>,
}

impl HuffmanTable {
    fn parse(
        counts: &[u8; 16],
        symbols: &[u8],
        lookup: &mut [HuffmanEntry],
    ) -> Result<Self, String> {
        lookup.fill(HuffmanEntry::INVALID);

        let mut code = 0_u32;
        let mut symbol_index = 0;
        for (length_index, &count) in counts.iter().enumerate() {
            let length = length_index + 1;
            for _ in 0..count {
                if code >= (1_u32 << length) {
                    return Err("lossless JPEG Huffman code lengths are invalid".to_owned());
                }
                let symbol = *symbols
                    .get(symbol_index)
                    .ok_or_else(|| "lossless JPEG Huffman symbol table is truncated".to_owned())?;
                let shift = HUFFMAN_LOOKAHEAD_BITS - length;
                let start = usize::try_from(code)
                    .map_err(|_| "lossless JPEG Huffman code is too large".to_owned())?
                    << shift;
                let end = start + (1 << shift);
                lookup[start..end].fill(HuffmanEntry {
                    symbol,
                    bits: u8::try_from(length)
                        .map_err(|_| "lossless JPEG Huffman code is too long".to_owned())?,
                });
                code += 1;
                symbol_index += 1;
            }
            code <<= 1;
        }

        Ok(Self {
            lookup: lookup.into(),
        })
    }

    fn decode(&self, bits: &mut BitReader<'_>) -> Result<u8, String> {
        let entry = self.lookup[usize::from(bits.peek(16))];
        if entry.bits == 0 {
            return Err("lossless JPEG contains an invalid Huffman code".to_owned());
        }
        bits.skip(usize::from(entry.bits));
        Ok(entry.symbol)
    }
}

struct Frame {
    width: usize,
    height: usize,
    precision: u8,
    components: Vec<Component>,
}

struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    buffer: u64,
    buffered_bits: u8,
    ended: bool,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            buffer: 0,
            buffered_bits: 0,
            ended: false,
        }
    }

    fn peek(&mut self, count: u8) -> u16 {
        while self.buffered_bits < count {
            let byte = self.next_byte();
            self.buffer = (self.buffer << 8) | u64::from(byte);
            self.buffered_bits += 8;
        }
        let shift = self.buffered_bits - count;
        let mask = (1_u64 << count) - 1;
        u16::try_from((self.buffer >> shift) & mask).unwrap_or_default()
    }

    fn skip(&mut self, count: usize) {
        self.buffered_bits = self
            .buffered_bits
            .saturating_sub(u8::try_from(count).expect("lossless JPEG bit counts are at most 16"));
        self.buffer &= (1_u64 << self.buffered_bits) - 1;
    }

    fn next_byte(&mut self) -> u8 {
        if self.ended || self.position >= self.bytes.len() {
            self.ended = true;
            return 0;
        }

        let byte = self.bytes[self.position];
        if byte != MARKER_PREFIX {
            self.position += 1;
            return byte;
        }

        if self.bytes.get(self.position + 1) == Some(&0) {
            self.position += 2;
            return byte;
        }

        self.ended = true;
        0
    }
}

struct HeaderReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> HeaderReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn u8(&mut self) -> Result<u8, String> {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| "lossless JPEG header is truncated".to_owned())?;
        self.position += 1;
        Ok(byte)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let high = u16::from(self.u8()?);
        let low = u16::from(self.u8()?);
        Ok((high << 8) | low)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| "lossless JPEG segment length overflow".to_owned())?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| "lossless JPEG segment is truncated".to_owned())?;
        self.position = end;
        Ok(bytes)
    }

    fn marker(&mut self) -> Result<u8, String> {
        while self.u8()? != MARKER_PREFIX {}
        loop {
            let marker = self.u8()?;
            if marker != MARKER_PREFIX {
                return Ok(marker);
            }
        }
    }
}

pub(super) fn decode(
    source: &[u8],
    expected_width: usize,
    expected_height: usize,
) -> Result<Vec<u16>, String> {
    let mut header = HeaderReader::new(source);
    if header.marker()? != MARKER_SOI {
        return Err("lossless JPEG stream does not start with SOI".to_owned());
    }

    let mut frame = None;
    let mut tables: [Option<HuffmanTable>; 4] = std::array::from_fn(|_| None);
    let (predictor, point_transform) = loop {
        let marker = header.marker()?;
        match marker {
            MARKER_SOF3 => frame = Some(parse_frame(&mut header)?),
            MARKER_DHT => parse_huffman_tables(&mut header, &mut tables)?,
            MARKER_SOS => {
                let frame = frame
                    .as_mut()
                    .ok_or_else(|| "lossless JPEG SOS appears before SOF3".to_owned())?;
                break parse_scan(&mut header, frame)?;
            }
            MARKER_EOI => {
                return Err("lossless JPEG reached EOI before SOS".to_owned());
            }
            MARKER_PREFIX | MARKER_SOI => {
                return Err(format!(
                    "lossless JPEG contains unexpected marker 0x{marker:02x}"
                ));
            }
            _ => {
                let length = usize::from(header.u16()?);
                if length < 2 {
                    return Err("lossless JPEG segment length is invalid".to_owned());
                }
                header.take(length - 2)?;
            }
        }
    };

    let frame = frame.expect("SOF3 is parsed before the scan starts");
    let output_width = frame
        .width
        .checked_mul(frame.components.len())
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if output_width < expected_width || frame.height < expected_height {
        return Err(format!(
            "lossless JPEG frame is {}x{}, smaller than expected {expected_width}x{expected_height}",
            output_width, frame.height
        ));
    }
    if !(1..=7).contains(&predictor) {
        return Err(format!(
            "lossless JPEG predictor {predictor} is unsupported"
        ));
    }
    if point_transform >= frame.precision {
        return Err("lossless JPEG point transform is invalid".to_owned());
    }

    let entropy = header
        .bytes
        .get(header.position..)
        .ok_or_else(|| "lossless JPEG entropy stream is missing".to_owned())?;
    decode_scan(
        entropy,
        &frame,
        &tables,
        predictor,
        point_transform,
        output_width,
    )
}

fn parse_frame(header: &mut HeaderReader<'_>) -> Result<Frame, String> {
    let length = usize::from(header.u16()?);
    let precision = header.u8()?;
    let height = usize::from(header.u16()?);
    let width = usize::from(header.u16()?);
    let component_count = usize::from(header.u8()?);
    if length != 8 + component_count * 3 {
        return Err("lossless JPEG SOF3 length is invalid".to_owned());
    }
    if !(1..=4).contains(&component_count) {
        return Err(format!(
            "lossless JPEG component count {component_count} is unsupported"
        ));
    }
    if !(2..=16).contains(&precision) {
        return Err(format!(
            "lossless JPEG precision {precision} is unsupported"
        ));
    }
    if width == 0 || height == 0 {
        return Err("lossless JPEG dimensions must not be zero".to_owned());
    }

    let mut components = Vec::with_capacity(component_count);
    for _ in 0..component_count {
        let id = header.u8()?;
        let sampling = header.u8()?;
        let _quantization = header.u8()?;
        if sampling != 0x11 {
            return Err(format!(
                "lossless JPEG sampling {sampling:#04x} is unsupported"
            ));
        }
        components.push(Component {
            id,
            huffman_table: 0,
        });
    }

    Ok(Frame {
        width,
        height,
        precision,
        components,
    })
}

fn parse_huffman_tables(
    header: &mut HeaderReader<'_>,
    tables: &mut [Option<HuffmanTable>; 4],
) -> Result<(), String> {
    let segment_length = usize::from(header.u16()?);
    if segment_length < 2 {
        return Err("lossless JPEG DHT length is invalid".to_owned());
    }
    let mut remaining = segment_length - 2;
    while remaining > 0 {
        let selector = header.u8()?;
        remaining -= 1;
        let class = selector >> 4;
        let table_index = usize::from(selector & 0x0f);
        if class != 0 || table_index >= tables.len() {
            return Err(format!(
                "lossless JPEG Huffman selector {selector:#04x} is unsupported"
            ));
        }
        let counts: [u8; 16] = header
            .take(16)?
            .try_into()
            .expect("the Huffman count slice has a fixed length");
        remaining -= 16;
        let symbol_count = counts
            .iter()
            .map(|&count| usize::from(count))
            .sum::<usize>();
        if symbol_count > 256 || symbol_count > remaining {
            return Err("lossless JPEG DHT symbol table is invalid".to_owned());
        }
        let symbols = header.take(symbol_count)?;
        remaining -= symbol_count;
        let mut lookup = vec![HuffmanEntry::INVALID; HUFFMAN_LOOKAHEAD_SIZE];
        tables[table_index] = Some(HuffmanTable::parse(&counts, symbols, &mut lookup)?);
    }
    Ok(())
}

fn parse_scan(header: &mut HeaderReader<'_>, frame: &mut Frame) -> Result<(u8, u8), String> {
    let length = usize::from(header.u16()?);
    if length != 6 + frame.components.len() * 2 {
        return Err("lossless JPEG SOS length is invalid".to_owned());
    }
    let component_count = usize::from(header.u8()?);
    if component_count != frame.components.len() {
        return Err("lossless JPEG SOS component count does not match SOF3".to_owned());
    }
    for _ in 0..component_count {
        let selector = header.u8()?;
        let table_selector = header.u8()?;
        let table_index = usize::from(table_selector >> 4);
        if table_selector & 0x0f != 0 || table_index >= 4 {
            return Err(format!(
                "lossless JPEG scan selector {table_selector:#04x} is unsupported"
            ));
        }
        let component = frame
            .components
            .iter_mut()
            .find(|component| component.id == selector)
            .ok_or_else(|| format!("lossless JPEG scan component {selector} is invalid"))?;
        component.huffman_table = table_index;
    }
    let predictor = header.u8()?;
    let end_of_spectral = header.u8()?;
    let point_transform = header.u8()?;
    if end_of_spectral != 0 || point_transform & 0xf0 != 0 {
        return Err("lossless JPEG scan transformation is unsupported".to_owned());
    }
    Ok((predictor, point_transform & 0x0f))
}

fn decode_scan(
    entropy: &[u8],
    frame: &Frame,
    tables: &[Option<HuffmanTable>; 4],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
) -> Result<Vec<u16>, String> {
    let mut components = Vec::with_capacity(frame.components.len());
    for component in &frame.components {
        components.push(
            tables[component.huffman_table]
                .as_ref()
                .ok_or_else(|| "lossless JPEG references a missing Huffman table".to_owned())?,
        );
    }

    let sample_count = output_width
        .checked_mul(frame.height)
        .ok_or_else(|| "lossless JPEG sample count overflow".to_owned())?;
    let mut output = vec![0_u16; sample_count];
    let mut bits = BitReader::new(entropy);
    let initial = 1_i32 << (frame.precision - point_transform - 1);
    let component_count = frame.components.len();

    for (component_index, table) in components.iter().enumerate() {
        output[component_index] = add_prediction(initial, table.decode(&mut bits)?, &mut bits)?;
    }

    for row in 0..frame.height {
        let first_column = usize::from(row == 0);
        for column in first_column..frame.width {
            for (component_index, table) in components.iter().enumerate() {
                let index = row * output_width + column * component_count + component_index;
                let prediction = predict(
                    &output,
                    output_width,
                    row,
                    column,
                    component_index,
                    component_count,
                    predictor,
                );
                output[index] = add_prediction(prediction, table.decode(&mut bits)?, &mut bits)?;
            }
        }
    }

    Ok(output)
}

fn predict(
    output: &[u16],
    output_width: usize,
    row: usize,
    column: usize,
    component: usize,
    components: usize,
    predictor: u8,
) -> i32 {
    let index = row * output_width + column * components + component;
    if column == 0 {
        debug_assert!(row > 0, "the first scan sample is initialized separately");
        return i32::from(output[index - output_width]);
    }

    let left = i32::from(output[index - components]);
    if row == 0 {
        return left;
    }

    let above = i32::from(output[index - output_width]);
    let above_left = i32::from(output[index - output_width - components]);
    match predictor {
        1 => left,
        2 => above,
        3 => above_left,
        4 => left + above - above_left,
        5 => left + ((above - above_left) >> 1),
        6 => above + ((left - above_left) >> 1),
        7 => (left + above) >> 1,
        _ => unreachable!("the predictor is validated before scanning"),
    }
}

fn add_prediction(prediction: i32, symbol: u8, bits: &mut BitReader<'_>) -> Result<u16, String> {
    let difference = match symbol {
        0 => 0,
        1..=16 => {
            let bit_count = symbol;
            let raw = i32::from(bits.peek(bit_count));
            bits.skip(usize::from(bit_count));
            let threshold = 1_i32 << (bit_count - 1);
            if raw < threshold {
                raw - ((1_i32 << bit_count) - 1)
            } else {
                raw
            }
        }
        _ => {
            return Err(format!(
                "lossless JPEG difference class {symbol} is unsupported"
            ));
        }
    };
    Ok((prediction.wrapping_add(difference) & 0xffff) as u16)
}

#[cfg(test)]
mod tests {
    use super::predict;

    #[test]
    fn all_lossless_jpeg_predictors_use_the_expected_neighbors() {
        let mut output = vec![0_u16; 9];
        output[0] = 4;
        output[1] = 20;
        output[3] = 10;

        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 1), 10);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 2), 20);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 3), 4);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 4), 26);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 5), 18);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 6), 23);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 7), 15);
    }
}
