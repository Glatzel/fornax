use fornax_codec::CodecError;
use fornax_codec::huffman::{BitReader, HuffmanTable};

#[test]
fn assigns_canonical_prefix_codes_in_symbol_order() {
    let table = HuffmanTable::from_counts(
        &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[5, 1, 2, 3, 4],
    )
    .unwrap();

    // Codes are 0, 100, 101, 110, 111.
    let mut reader = BitReader::new(&[0b0100_1011, 0b1011_1000]);

    assert_eq!(table.decode_symbol(&mut reader).unwrap(), 5);
    assert_eq!(table.decode_symbol(&mut reader).unwrap(), 1);
    assert_eq!(table.decode_symbol(&mut reader).unwrap(), 2);
    assert_eq!(table.decode_symbol(&mut reader).unwrap(), 3);
    assert_eq!(table.decode_symbol(&mut reader).unwrap(), 4);
}

#[test]
fn rejects_over_subscribed_codes() {
    let error = HuffmanTable::from_counts(
        &[3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[1, 2, 3],
    )
    .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn rejects_incomplete_codes() {
    let error =
        HuffmanTable::from_counts(&[0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], &[1, 2])
            .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn rejects_code_space_overflow_after_sixteen_bits() {
    let error = HuffmanTable::from_counts(
        &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3],
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
    )
    .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn rejects_symbol_count_that_disagrees_with_counts() {
    let error = HuffmanTable::from_counts(
        &[1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[5, 1, 2, 3],
    )
    .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn bit_reader_reads_msb_first_across_byte_boundaries() {
    let mut reader = BitReader::new(&[0xb2, 0x6c]);

    assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    assert_eq!(reader.read_bits(7).unwrap(), 0b1001001);
    assert_eq!(reader.read_bits(6).unwrap(), 0b101100);
}

#[test]
fn bit_reader_rejects_reads_past_end_of_data() {
    let mut reader = BitReader::new(&[0]);
    let error = reader.read_bits(9).unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn lookup_and_slow_paths_match_naive_decoder_on_random_streams() {
    let counts = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2];
    let symbols = [
        19, 3, 91, 7, 42, 255, 0, 16, 8, 4, 2, 31, 63, 127, 200, 240, 17,
    ];
    let table = HuffmanTable::from_counts(&counts, &symbols).unwrap();
    let codes = canonical_codes(&counts, &symbols);
    let mut state = 0x9e37_79b9_u32;
    let mut source = Vec::new();
    let mut expected = Vec::new();

    for _ in 0..10_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let (symbol, _, _) = codes[state as usize % codes.len()];
        expected.push(symbol);
        source.extend(symbol_bits(symbol, &codes));
    }
    let mut source = pack_bits(&source);
    source.extend([0xff; 3]);

    let mut fast = BitReader::new(&source);
    let mut slow = BitReader::new(&source);
    for expected in expected {
        assert_eq!(table.decode_symbol(&mut fast).unwrap(), expected);
        assert_eq!(
            decode_naive(&counts, &symbols, &mut slow).unwrap(),
            expected
        );
    }
}

fn canonical_codes(counts: &[u8; 16], symbols: &[u8]) -> Vec<(u8, u16, u8)> {
    let mut code = 0_u32;
    let mut offset = 0;
    let mut output = Vec::new();
    for (index, count) in counts.iter().copied().enumerate() {
        code <<= 1;
        for _ in 0..count {
            output.push((symbols[offset], code as u16, index as u8 + 1));
            code += 1;
            offset += 1;
        }
    }
    output
}

fn symbol_bits(symbol: u8, codes: &[(u8, u16, u8)]) -> Vec<bool> {
    let (_, code, length) = codes.iter().find(|(value, _, _)| *value == symbol).unwrap();
    (0..*length)
        .rev()
        .map(|shift| code & (1 << shift) != 0)
        .collect()
}

fn pack_bits(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk.iter().enumerate().fold(0_u8, |byte, (index, bit)| {
                byte | (u8::from(*bit) << (7 - index))
            })
        })
        .collect()
}

fn decode_naive(
    counts: &[u8; 16],
    symbols: &[u8],
    reader: &mut BitReader<'_>,
) -> Result<u8, CodecError> {
    let mut code = 0_u32;
    let mut first = 0_u32;
    let mut offset = 0_usize;
    for count in counts.iter().copied() {
        code = (code << 1) | reader.read_bits(1)?;
        if code >= first && code < first + u32::from(count) {
            return Ok(symbols[offset + (code - first) as usize]);
        }
        offset += usize::from(count);
        first = (first + u32::from(count)) << 1;
    }
    Err(CodecError::InvalidInput("invalid Huffman code".into()))
}
