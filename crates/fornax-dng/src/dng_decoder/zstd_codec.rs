use std::io::Read;

use structured_zstd::encoding::{CompressionLevel, compress_to_vec};

pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, String> {
    Ok(compress_to_vec(data, CompressionLevel::from_level(level)))
}

pub fn decompress(encoded: &[u8], max_output: usize) -> Result<Vec<u8>, String> {
    let decoder = structured_zstd::decoding::StreamingDecoder::new(encoded);
    let limit = u64::try_from(max_output)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut decoded = Vec::new();
    decoder
        .take(limit)
        .read_to_end(&mut decoded)
        .map_err(|error| format!("Zstandard payload is invalid: {error}"))?;
    if decoded.len() > max_output {
        return Err("Zstandard payload exceeds the declared tile size".into());
    }
    Ok(decoded)
}
