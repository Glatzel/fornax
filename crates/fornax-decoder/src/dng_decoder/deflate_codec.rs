use std::io::Read;

use flate2::read::ZlibDecoder;

pub(super) fn inflate(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(encoded);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| format!("Deflate payload is invalid: {error}"))?;
    Ok(decoded)
}

pub(super) fn reverse_predictor(
    payload: &mut [u8],
    predictor: u16,
    tile_width: usize,
    tile_height: usize,
    bits: u16,
    sample_format: u64,
    little_endian: bool,
) -> Result<(), String> {
    if predictor == 1 {
        return Ok(());
    }
    let bytes_per_sample = usize::try_from(bits.div_ceil(8))
        .map_err(|_| "Deflate sample width exceeds the platform address space".to_owned())?;
    let row_bytes = tile_width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| "Deflate row size overflow".to_owned())?;
    let needed = row_bytes
        .checked_mul(tile_height)
        .ok_or_else(|| "Deflate payload size overflow".to_owned())?;
    if payload.len() < needed {
        return Err("Deflate payload is truncated".to_owned());
    }

    match predictor {
        2 => {
            if sample_format != 1 || !matches!(bits, 8 | 16 | 32) {
                return Err("horizontal Deflate prediction requires 8/16/32-bit integers".into());
            }
            for row in payload[..needed].chunks_exact_mut(row_bytes) {
                for sample in 1..tile_width {
                    let previous = (sample - 1) * bytes_per_sample;
                    let current = sample * bytes_per_sample;
                    match bytes_per_sample {
                        1 => row[current] = row[current].wrapping_add(row[previous]),
                        2 => write_u16(
                            row,
                            current,
                            read_u16(row, previous, little_endian).wrapping_add(read_u16(
                                row,
                                current,
                                little_endian,
                            )),
                            little_endian,
                        ),
                        4 => write_u32(
                            row,
                            current,
                            read_u32(row, previous, little_endian).wrapping_add(read_u32(
                                row,
                                current,
                                little_endian,
                            )),
                            little_endian,
                        ),
                        _ => unreachable!("the byte width is validated above"),
                    }
                }
            }
        }
        3 | 34_894 | 34_895 => {
            if sample_format != 3 || !matches!(bits, 16 | 24 | 32) {
                return Err(
                    "floating-point Deflate prediction requires 16/24/32-bit floats".into(),
                );
            }
            let factor = match predictor {
                3 => 1,
                34_894 => 2,
                34_895 => 4,
                _ => unreachable!("the predictor is matched above"),
            };
            for row in payload[..needed].chunks_exact_mut(row_bytes) {
                for index in factor..row_bytes {
                    row[index] = row[index].wrapping_add(row[index - factor]);
                }
                let shuffled = row.to_vec();
                for sample in 0..tile_width {
                    for byte in 0..bytes_per_sample {
                        row[sample * bytes_per_sample + byte] =
                            shuffled[sample + byte * tile_width];
                    }
                }
            }
        }
        _ => return Err(format!("Deflate predictor {predictor} is invalid")),
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize, little_endian: bool) -> u16 {
    let value = [bytes[offset], bytes[offset + 1]];
    if little_endian {
        u16::from_le_bytes(value)
    } else {
        u16::from_be_bytes(value)
    }
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16, little_endian: bool) {
    let value = if little_endian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    bytes[offset..offset + 2].copy_from_slice(&value);
}

fn read_u32(bytes: &[u8], offset: usize, little_endian: bool) -> u32 {
    let value = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    if little_endian {
        u32::from_le_bytes(value)
    } else {
        u32::from_be_bytes(value)
    }
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32, little_endian: bool) {
    let value = if little_endian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    bytes[offset..offset + 4].copy_from_slice(&value);
}
