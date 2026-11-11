//! Canonical Huffman tables and MSB-first bit reading.
//!
//! The table builder accepts the same 16 code-length counts used by many raw
//! formats. Codes are assigned canonically in symbol order. A fixed lookup
//! table handles common short codes, while codes longer than the lookup width
//! fall back to a bounded bit-by-bit search.

use crate::CodecError;
use crate::error::invalid;

const FAST_BITS: u8 = 10;
const FAST_SIZE: usize = 1 << FAST_BITS;

#[derive(Clone, Copy, Debug)]
struct LookupEntry {
    length: u8,
    symbol: u8,
}

impl LookupEntry {
    const EMPTY: Self = Self {
        length: 0,
        symbol: 0,
    };
}

/// A complete canonical Huffman table with at most 16-bit codes.
///
/// # Examples
///
/// ```
/// use fornax_codec::huffman::{BitReader, HuffmanTable};
///
/// let table = HuffmanTable::from_counts(
///     &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
///     &[5, 1, 2, 3, 4],
/// )
/// .unwrap();
/// let mut bits = BitReader::new(&[0b0100_1011]);
/// assert_eq!(table.decode_symbol(&mut bits).unwrap(), 5);
/// ```
#[derive(Debug)]
pub struct HuffmanTable {
    counts: [u8; 16],
    symbols: Box<[u8]>,
    first_codes: [u32; 16],
    offsets: [usize; 16],
    lookup: Box<[LookupEntry]>,
}

impl HuffmanTable {
    /// Builds a complete canonical table from code-length counts and symbols.
    ///
    /// `counts[0]` is the number of one-bit codes and `counts[15]` is the
    /// number of 16-bit codes. `symbols` must contain exactly the total count
    /// in canonical order.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidInput`] when the symbol count disagrees,
    /// no codes are present, the code space is over-subscribed, or the codes
    /// do not form a complete prefix code.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::HuffmanTable;
    ///
    /// let table = HuffmanTable::from_counts(
    ///     &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ///     &[5, 1, 2, 3, 4],
    /// )
    /// .unwrap();
    /// assert_eq!(table.symbol_count(), 5);
    /// ```
    pub fn from_counts(counts: &[u8; 16], symbols: &[u8]) -> Result<Self, CodecError> {
        let total = counts.iter().try_fold(0_usize, |total, count| {
            total
                .checked_add(usize::from(*count))
                .ok_or_else(|| invalid("Huffman symbol count overflow"))
        })?;
        if total == 0 {
            return Err(invalid("Huffman table has no symbols"));
        }
        if total != symbols.len() {
            return Err(invalid(format!(
                "Huffman table has {total} codes but {} symbols",
                symbols.len()
            )));
        }

        let mut first_codes = [0_u32; 16];
        let mut offsets = [0_usize; 16];
        let mut lookup = vec![LookupEntry::EMPTY; FAST_SIZE];
        let mut code = 0_u32;
        let mut offset = 0_usize;

        for (index, count) in counts.iter().copied().enumerate() {
            let length = index as u8 + 1;
            code <<= 1;
            let end = code
                .checked_add(u32::from(count))
                .ok_or_else(|| invalid("Huffman code space overflow"))?;
            let capacity = 1_u32 << length;
            if end > capacity {
                return Err(invalid(format!(
                    "Huffman table is over-subscribed at {length}-bit codes"
                )));
            }
            first_codes[index] = code;
            offsets[index] = offset;

            if length <= FAST_BITS {
                let suffix_count = 1_usize << (FAST_BITS - length);
                for value in code..end {
                    let start = (value as usize) << (FAST_BITS - length);
                    lookup[start..start + suffix_count].fill(LookupEntry {
                        length,
                        symbol: symbols[offset + (value - code) as usize],
                    });
                }
            }

            code = end;
            offset += usize::from(count);
        }

        if code != 1_u32 << 16 {
            return Err(invalid("Huffman table is incomplete"));
        }

        Ok(Self {
            counts: *counts,
            symbols: symbols.into(),
            first_codes,
            offsets,
            lookup: lookup.into_boxed_slice(),
        })
    }

    /// Decodes one symbol and consumes its code bits.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidInput`] when the stream ends inside a code
    /// or the fetched bits do not identify a symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::{BitReader, HuffmanTable};
    ///
    /// let table = HuffmanTable::from_counts(
    ///     &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ///     &[5, 1, 2, 3, 4],
    /// )
    /// .unwrap();
    /// let mut bits = BitReader::new(&[0b1000_0000]);
    /// assert_eq!(table.decode_symbol(&mut bits).unwrap(), 1);
    /// ```
    pub fn decode_symbol(&self, bits: &mut BitReader<'_>) -> Result<u8, CodecError> {
        if bits.remaining_bits() >= usize::from(FAST_BITS) {
            let entry = self.lookup[bits.peek_bits(FAST_BITS)? as usize];
            if entry.length != 0 {
                bits.read_bits(entry.length)?;
                return Ok(entry.symbol);
            }
        }

        let mut code = 0_u32;
        for index in 0..self.counts.len() {
            code = (code << 1) | bits.read_bits(1)?;
            let count = u32::from(self.counts[index]);
            let first = self.first_codes[index];
            if code >= first && code < first + count {
                return Ok(self.symbols[self.offsets[index] + (code - first) as usize]);
            }
        }
        Err(invalid("invalid Huffman code"))
    }

    /// Returns the number of symbols represented by the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::HuffmanTable;
    ///
    /// let table = HuffmanTable::from_counts(
    ///     &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ///     &[5, 1, 2, 3, 4],
    /// )
    /// .unwrap();
    /// assert_eq!(table.symbol_count(), 5);
    /// ```
    pub fn symbol_count(&self) -> usize { self.symbols.len() }
}

/// An MSB-first bit reader over an immutable byte stream.
///
/// The reader buffers up to 64 bits and does not perform byte stuffing.
///
/// # Examples
///
/// ```
/// use fornax_codec::huffman::BitReader;
///
/// let mut bits = BitReader::new(&[0b1010_1100]);
/// assert_eq!(bits.read_bits(3).unwrap(), 0b101);
/// assert_eq!(bits.read_bits(3).unwrap(), 0b011);
/// ```
pub struct BitReader<'a> {
    source: &'a [u8],
    next_byte: usize,
    buffer: u64,
    buffered: u8,
}

impl<'a> BitReader<'a> {
    /// Creates a reader positioned at the most-significant bit of `source`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::BitReader;
    ///
    /// let mut bits = BitReader::new(&[0x80]);
    /// assert_eq!(bits.read_bits(1).unwrap(), 1);
    /// ```
    pub const fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            next_byte: 0,
            buffer: 0,
            buffered: 0,
        }
    }

    /// Reads up to 32 bits MSB-first.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidInput`] when `count` exceeds 32 or the
    /// stream does not contain enough remaining bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::BitReader;
    ///
    /// let mut bits = BitReader::new(&[0b1100_0000]);
    /// assert_eq!(bits.read_bits(2).unwrap(), 3);
    /// ```
    pub fn read_bits(&mut self, count: u8) -> Result<u32, CodecError> {
        if count > 32 {
            return Err(invalid("bit read exceeds 32 bits"));
        }
        if count == 0 {
            return Ok(0);
        }
        self.fill(count);
        if self.buffered < count {
            return Err(invalid("Huffman bitstream is exhausted"));
        }
        let value = self.buffer >> (64 - count);
        self.buffer <<= count;
        self.buffered -= count;
        Ok(value as u32)
    }

    /// Returns the number of bits not yet consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::huffman::BitReader;
    ///
    /// assert_eq!(BitReader::new(&[0; 2]).remaining_bits(), 16);
    /// ```
    pub const fn remaining_bits(&self) -> usize {
        (self.source.len() - self.next_byte) * 8 + self.buffered as usize
    }

    fn peek_bits(&mut self, count: u8) -> Result<u32, CodecError> {
        if count == 0 {
            return Ok(0);
        }
        self.fill(count);
        if self.buffered < count {
            return Err(invalid("Huffman bitstream is exhausted"));
        }
        Ok((self.buffer >> (64 - count)) as u32)
    }

    fn fill(&mut self, required: u8) {
        while self.buffered < required && self.next_byte < self.source.len() {
            self.buffer |= u64::from(self.source[self.next_byte]) << (56 - self.buffered);
            self.next_byte += 1;
            self.buffered += 8;
        }
    }
}
