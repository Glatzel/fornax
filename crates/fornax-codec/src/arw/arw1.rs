use crate::error::invalid;
use crate::request::SampleFormat;
use crate::samples::{Geometry, buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, SampleBuffer};

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("Sony ARW1 requires integer samples"));
    }
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    if geometry.chunk_width != geometry.width || geometry.chunk_height != geometry.height {
        return Err(invalid("Sony ARW1 requires one whole-image chunk"));
    }
    let encoded = chunk(request.source, request.chunks[0])?;
    let sample_count = geometry
        .width
        .checked_mul(geometry.height)
        .ok_or_else(|| invalid("Sony ARW1 sample count overflows"))?;
    let mut output = vec![0_u16; sample_count];
    let mut bits = BitReader::new(encoded);
    let mut sum = 0_i32;

    for column in (0..geometry.width).rev() {
        let mut row = 0_usize;
        while row <= geometry.height {
            if row == geometry.height {
                row = 1;
            }
            let mut length = 4_u32 - bits.read(2)?;
            if length == 3 && bits.read(1)? != 0 {
                length = 0;
            } else if length == 4 {
                let zeros = bits.peek(13).leading_zeros() - 19;
                length += zeros;
                bits.skip(usize::try_from(zeros + 1).expect("at most 13 bits"));
            }
            let difference = bits.read_signed(length)?;
            sum = sum.wrapping_add(difference);
            if length > 0 && difference & (1 << (length - 1)) == 0 {
                sum -= (1 << length) - 1;
            }
            output[row * geometry.width + column] = sum as u16;
            row += 2;
        }
    }

    buffer_u16(geometry.width, geometry.height, output)
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, bit: 0 } }

    fn peek(&self, count: u8) -> u32 {
        let mut value = 0_u32;
        for index in 0..usize::from(count) {
            value = (value << 1) | u32::from(self.bit_at(self.bit + index));
        }
        value
    }

    fn read(&mut self, count: u8) -> Result<u32, CodecError> {
        let value = self.peek(count);
        self.skip(usize::from(count));
        Ok(value)
    }

    fn read_signed(&mut self, count: u32) -> Result<i32, CodecError> {
        let count =
            u8::try_from(count).map_err(|_| invalid("Sony ARW1 difference exceeds 8 bits"))?;
        Ok(self.read(count)? as i32)
    }

    const fn skip(&mut self, count: usize) { self.bit = self.bit.saturating_add(count); }

    fn bit_at(&self, index: usize) -> u8 {
        self.bytes
            .get(index / 8)
            .map_or(0, |byte| (byte >> (7 - index % 8)) & 1)
    }
}

#[cfg(test)]
mod tests {
    use super::BitReader;

    #[test]
    fn reader_zero_fills_past_the_end() {
        let mut bits = BitReader::new(&[0b1010_0000]);
        assert_eq!(bits.read(4).unwrap(), 0b1010);
        assert_eq!(bits.read(8).unwrap(), 0);
    }
}
