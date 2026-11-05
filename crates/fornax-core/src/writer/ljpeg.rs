const MARKER_PREFIX: u8 = 0xff;
const MARKER_SOF3: u8 = 0xc3;
const MARKER_DHT: u8 = 0xc4;
const MARKER_SOI: u8 = 0xd8;
const MARKER_EOI: u8 = 0xd9;
const MARKER_SOS: u8 = 0xda;
const HUFFMAN_BITS: u8 = 5;

pub(super) fn encode(
    samples: &[u16],
    width: u16,
    height: u16,
    bits: u8,
) -> Result<Vec<u8>, String> {
    if !(2..=16).contains(&bits) {
        return Err(format!("lossless JPEG bit depth {bits} is invalid"));
    }
    let expected = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(|| "lossless JPEG tile size overflow".to_owned())?;
    if samples.len() != expected {
        return Err(format!(
            "lossless JPEG input has {} samples, expected {expected}",
            samples.len()
        ));
    }

    let mut encoded = Vec::with_capacity(samples.len() * 2);
    write_marker(&mut encoded, MARKER_SOI);
    write_marker(&mut encoded, MARKER_SOF3);
    push_u16(&mut encoded, 11);
    encoded.push(bits);
    push_u16(&mut encoded, height);
    push_u16(&mut encoded, width);
    encoded.extend_from_slice(&[1, 1, 0x11, 0]);

    write_marker(&mut encoded, MARKER_DHT);
    push_u16(&mut encoded, 36);
    encoded.push(0);
    encoded.extend_from_slice(&[0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    encoded.extend(0_u8..=16);

    write_marker(&mut encoded, MARKER_SOS);
    push_u16(&mut encoded, 8);
    encoded.extend_from_slice(&[1, 1, 0, 1, 0, 0]);

    let modulus = 1_i32 << bits;
    let half = modulus >> 1;
    let initial = half as u16;
    let mut writer = BitWriter::new(&mut encoded);
    for row in 0..usize::from(height) {
        for column in 0..usize::from(width) {
            let index = row * usize::from(width) + column;
            let prediction = if index == 0 {
                initial
            } else if column == 0 {
                samples[index - usize::from(width)]
            } else {
                samples[index - 1]
            };
            let raw_difference = i32::from(samples[index]) - i32::from(prediction);
            let difference = (raw_difference + half).rem_euclid(modulus) - half;
            let category = if difference == 0 {
                0
            } else {
                difference.unsigned_abs().ilog2() + 1
            };
            let category = u8::try_from(category).expect("lossless JPEG category is at most 16");
            writer.write(u32::from(category), HUFFMAN_BITS);
            if category != 0 && category != bits {
                let value = if difference < 0 {
                    difference + ((1_i32 << category) - 1)
                } else {
                    difference
                };
                writer.write(
                    u32::try_from(value).expect("lossless JPEG amplitude is non-negative"),
                    category,
                );
            }
        }
    }
    writer.finish();
    write_marker(&mut encoded, MARKER_EOI);
    Ok(encoded)
}

fn write_marker(encoded: &mut Vec<u8>, marker: u8) {
    encoded.extend_from_slice(&[MARKER_PREFIX, marker]);
}

fn push_u16(encoded: &mut Vec<u8>, value: u16) {
    encoded.extend_from_slice(&value.to_be_bytes());
}

struct BitWriter<'a> {
    encoded: &'a mut Vec<u8>,
    current: u8,
    used: u8,
}

impl<'a> BitWriter<'a> {
    const fn new(encoded: &'a mut Vec<u8>) -> Self {
        Self {
            encoded,
            current: 0,
            used: 0,
        }
    }

    fn write(&mut self, value: u32, bits: u8) {
        for shift in (0..bits).rev() {
            self.current = (self.current << 1) | (((value >> shift) & 1) as u8);
            self.used += 1;
            if self.used == 8 {
                self.flush_byte();
            }
        }
    }

    fn finish(&mut self) {
        while self.used != 0 {
            self.write(1, 1);
        }
    }

    fn flush_byte(&mut self) {
        self.encoded.push(self.current);
        if self.current == MARKER_PREFIX {
            self.encoded.push(0);
        }
        self.current = 0;
        self.used = 0;
    }
}
