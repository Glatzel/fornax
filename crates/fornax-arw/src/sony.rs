use fornax_core::error::DecodeError;

pub fn decrypt(
    source: &[u8],
    offset: usize,
    length: usize,
    key: u32,
) -> Result<Vec<u8>, DecodeError> {
    let length = length / 4 * 4;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid("Sony private payload range overflows"))?;
    let encrypted = source
        .get(offset..end)
        .ok_or_else(|| invalid("Sony private payload is outside the source file"))?;

    let mut pad = [0_u32; 128];
    let mut state = key;
    for value in &mut pad[..4] {
        state = state.wrapping_mul(48_828_125).wrapping_add(1);
        *value = state;
    }
    pad[3] = (pad[3] << 1) | ((pad[0] ^ pad[2]) >> 31);
    for index in 4..127 {
        pad[index] =
            ((pad[index - 4] ^ pad[index - 2]) << 1) | ((pad[index - 3] ^ pad[index - 1]) >> 31);
    }
    for value in &mut pad[..127] {
        *value = value.to_be();
    }

    let mut output = Vec::with_capacity(length);
    for (index, bytes) in encrypted.as_chunks::<4>().0.iter().enumerate() {
        let slot = (index + 127) & 127;
        pad[slot] = pad[(slot + 1) & 127] ^ pad[(slot + 65) & 127];
        let encrypted = u32::from_le_bytes(*bytes);
        output.extend_from_slice(&(encrypted ^ pad[slot]).to_le_bytes());
    }
    Ok(output)
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

#[cfg(test)]
mod tests {
    use super::decrypt;

    #[test]
    fn repeated_runs_are_consistent_and_bounds_checked() {
        let source = (0_u8..64).collect::<Vec<_>>();
        let first = decrypt(&source, 0, 64, 1234).unwrap();
        let second = decrypt(&source, 0, 64, 1234).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, source);
        assert_eq!(decrypt(&source, 0, 5, 1234).unwrap().len(), 4);
        assert!(decrypt(&source, 60, 8, 1234).is_err());
    }
}
