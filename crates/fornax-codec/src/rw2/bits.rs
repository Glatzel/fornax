use crate::CodecError;
use crate::error::invalid;

pub struct LsbBits<'a> {
    bytes: &'a [u8],
    bit_len: usize,
    position: usize,
}

impl<'a> LsbBits<'a> {
    pub fn new(bytes: &'a [u8], bit_len: usize) -> Result<Self, CodecError> {
        let available = bytes
            .len()
            .checked_mul(8)
            .ok_or_else(|| invalid("Panasonic bit count overflow"))?;
        if bit_len > available {
            return Err(invalid("Panasonic bitstream is truncated"));
        }
        Ok(Self {
            bytes,
            bit_len,
            position: 0,
        })
    }

    pub fn read(&mut self, count: u32) -> Result<u32, CodecError> {
        if count > 32 {
            return Err(invalid("Panasonic bit read exceeds 32 bits"));
        }
        let count = usize::try_from(count).map_err(|_| invalid("Panasonic bit count overflow"))?;
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic bit position overflow"))?;
        if end > self.bit_len {
            return Err(invalid("Panasonic bitstream is truncated"));
        }
        let mut value = 0_u32;
        for bit in 0..count {
            let position = self.position + bit;
            let source = (self.bytes[position / 8] >> (position % 8)) & 1;
            value |= u32::from(source) << bit;
        }
        self.position = end;
        Ok(value)
    }

    pub fn peek_16(&self) -> u16 {
        let mut value = 0_u16;
        for bit in 0..16 {
            let position = self.position + bit;
            if position >= self.bit_len {
                break;
            }
            let source = (self.bytes[position / 8] >> (position % 8)) & 1;
            value |= u16::from(source) << bit;
        }
        value
    }

    pub fn skip(&mut self, count: usize) -> Result<(), CodecError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic bit position overflow"))?;
        if end > self.bit_len {
            return Err(invalid("Panasonic bitstream is truncated"));
        }
        self.position = end;
        Ok(())
    }
}

pub struct PanasonicV4Bits<'a> {
    bytes: &'a [u8],
    consumed: usize,
    split: bool,
}

impl<'a> PanasonicV4Bits<'a> {
    const BLOCK_BITS: usize = 0x4000 * 8;
    const BLOCK_BYTES: usize = 0x4000;
    const SPLIT_SHIFT: usize = 0x2008;

    pub fn new(bytes: &'a [u8], split: bool) -> Self {
        Self {
            bytes,
            consumed: 0,
            split,
        }
    }

    pub fn read(&mut self, count: usize) -> Result<u32, CodecError> {
        if count == 0 || count > 16 {
            return Err(invalid("Panasonic V4 bit read must be between 1 and 16"));
        }
        let remaining = Self::BLOCK_BITS
            .checked_sub(self.consumed % Self::BLOCK_BITS)
            .ok_or_else(|| invalid("Panasonic V4 bit position overflow"))?;
        if count > remaining {
            return Err(invalid("Panasonic V4 row crosses a bit-pump block"));
        }
        let block = self.consumed / Self::BLOCK_BITS;
        let in_block = self.consumed % Self::BLOCK_BITS;
        let bit = Self::BLOCK_BITS - in_block - count;
        let mut byte = (bit >> 3) ^ 0x3ff0;
        if self.split {
            byte = (byte + Self::BLOCK_BYTES - Self::SPLIT_SHIFT) % Self::BLOCK_BYTES;
        }
        let offset = block
            .checked_mul(Self::BLOCK_BYTES)
            .and_then(|offset| offset.checked_add(byte))
            .ok_or_else(|| invalid("Panasonic V4 byte offset overflow"))?;
        let pair = self
            .bytes
            .get(offset..offset.saturating_add(2))
            .ok_or_else(|| invalid("Panasonic V4 payload is truncated"))?;
        let word = u32::from(u16::from_le_bytes([pair[0], pair[1]]));
        let shift = u32::try_from(bit & 7).map_err(|_| invalid("Panasonic V4 shift overflow"))?;
        let mask = 0xffff_u32 >> (16 - count);
        self.consumed = self
            .consumed
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic V4 bit position overflow"))?;
        Ok((word >> shift) & mask)
    }
}
