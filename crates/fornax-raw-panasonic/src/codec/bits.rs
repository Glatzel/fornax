use crate::CodecError;
use crate::error::invalid;

pub struct LsbBits<'a> {
    bytes: &'a [u8],
    bit_len: usize,
    position: usize,
}

pub struct V8Bits<'a> {
    bytes: &'a [u8],
    bit_len: usize,
    position: usize,
}

impl<'a> V8Bits<'a> {
    pub fn new(bytes: &'a [u8], bit_len: usize) -> Result<Self, CodecError> {
        let available = bytes
            .len()
            .checked_mul(8)
            .ok_or_else(|| invalid("Panasonic V8 bit count overflow"))?;
        if bit_len > available {
            return Err(invalid("Panasonic V8 bitstream is truncated"));
        }
        Ok(Self {
            bytes,
            bit_len,
            position: 0,
        })
    }

    pub fn read(&mut self, count: u32) -> Result<u32, CodecError> {
        if count > 32 {
            return Err(invalid("Panasonic V8 bit read exceeds 32 bits"));
        }
        let count =
            usize::try_from(count).map_err(|_| invalid("Panasonic V8 bit count overflow"))?;
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic V8 bit position overflow"))?;
        if end > self.bit_len {
            return Err(invalid("Panasonic V8 bitstream is truncated"));
        }
        let value = self.bits_at(self.position, count).reverse_bits();
        let value = if count == 32 {
            value
        } else {
            value >> (32 - count)
        };
        self.position = end;
        Ok(value)
    }

    pub fn peek_16(&self) -> u16 {
        let count = (self.bit_len - self.position).min(16);
        if count == 0 {
            return 0;
        }
        self.bits_at(self.position, count)
            .reverse_bits()
            .wrapping_shr((32 - count) as u32) as u16
    }

    pub fn skip(&mut self, count: usize) -> Result<(), CodecError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic V8 bit position overflow"))?;
        if end > self.bit_len {
            return Err(invalid("Panasonic V8 bitstream is truncated"));
        }
        self.position = end;
        Ok(())
    }

    fn bits_at(&self, position: usize, count: usize) -> u32 {
        if count == 0 {
            return 0;
        }
        let byte = position / 8;
        let shift = position % 8;
        let bytes = (shift + count).div_ceil(8);
        let mut value = 0_u64;
        for index in 0..bytes {
            value |= u64::from(self.bytes[byte + index]) << (index * 8);
        }
        let value = (value >> shift) as u32;
        if count == 32 {
            value
        } else {
            value & ((1_u32 << count) - 1)
        }
    }
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
        let value = self.bits_at(self.position, count);
        self.position = end;
        Ok(value)
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

    fn bits_at(&self, position: usize, count: usize) -> u32 {
        if count == 0 {
            return 0;
        }
        let byte = position / 8;
        let shift = position % 8;
        let bytes = (shift + count).div_ceil(8);
        let mut value = 0_u64;
        for index in 0..bytes {
            value |= u64::from(self.bytes[byte + index]) << (index * 8);
        }
        let value = (value >> shift) as u32;
        if count == 32 {
            value
        } else {
            value & ((1_u32 << count) - 1)
        }
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

    pub const fn new(bytes: &'a [u8], split: bool) -> Self {
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
        let first = self.bytes.get(offset).copied().ok_or_else(|| {
            invalid(format!(
                "Panasonic V4 payload is truncated at byte {offset} of {}",
                self.bytes.len()
            ))
        })?;
        let second = self.bytes.get(offset + 1).copied().unwrap_or(0);
        let word = u32::from(u16::from_le_bytes([first, second]));
        let shift = u32::try_from(bit & 7).map_err(|_| invalid("Panasonic V4 shift overflow"))?;
        let mask = 0xffff_u32 >> (16 - count);
        self.consumed = self
            .consumed
            .checked_add(count)
            .ok_or_else(|| invalid("Panasonic V4 bit position overflow"))?;
        Ok((word >> shift) & mask)
    }
}
