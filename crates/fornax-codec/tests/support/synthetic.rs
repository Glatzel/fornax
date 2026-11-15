use fornax_codec::{
    ByteOrder, Chunk, EncodeCompression, EncodeRequest, ICodecEncode, Predictor, SampleBuffer,
    SampleFormat,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntheticPattern {
    Smooth,
    Noisy { seed: u64 },
}

#[allow(dead_code)]
pub trait FixtureSample: Copy {
    const BITS: u16;
    const FORMAT: SampleFormat;

    fn sample(word: u64) -> Self;

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer;
}

impl FixtureSample for u8 {
    const BITS: u16 = 8;
    const FORMAT: SampleFormat = SampleFormat::Integer;

    fn sample(word: u64) -> Self { word as Self }

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer {
        SampleBuffer::from_u8(width, height, samples).unwrap()
    }
}

impl FixtureSample for u16 {
    const BITS: u16 = 16;
    const FORMAT: SampleFormat = SampleFormat::Integer;

    fn sample(word: u64) -> Self { word as Self }

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer {
        SampleBuffer::from_u16(width, height, samples).unwrap()
    }
}

impl FixtureSample for u32 {
    const BITS: u16 = 32;
    const FORMAT: SampleFormat = SampleFormat::Integer;

    fn sample(word: u64) -> Self { word as Self }

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer {
        SampleBuffer::from_u32(width, height, samples).unwrap()
    }
}

impl FixtureSample for f32 {
    const BITS: u16 = 32;
    const FORMAT: SampleFormat = SampleFormat::Float;

    fn sample(word: u64) -> Self { (word & 0xffff) as Self / 65_535.0 }

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer {
        SampleBuffer::from_f32(width, height, samples).unwrap()
    }
}

impl FixtureSample for f64 {
    const BITS: u16 = 64;
    const FORMAT: SampleFormat = SampleFormat::Float;

    fn sample(word: u64) -> Self { (word & 0xffff) as Self / 65_535.0 }

    fn buffer(width: u32, height: u32, samples: Vec<Self>) -> SampleBuffer {
        SampleBuffer::from_f64(width, height, samples).unwrap()
    }
}

pub struct SyntheticFixture {
    pub image: SampleBuffer,
    pub source: Vec<u8>,
    pub chunks: Vec<Chunk>,
}

pub fn synthetic_image<S: FixtureSample>(
    width: u32,
    height: u32,
    pattern: SyntheticPattern,
) -> SampleBuffer {
    let count = usize::try_from(width)
        .unwrap()
        .checked_mul(usize::try_from(height).unwrap())
        .unwrap();
    let samples = (0..count)
        .map(|index| {
            S::sample(pattern_word(
                pattern,
                index,
                usize::try_from(width).unwrap(),
            ))
        })
        .collect();
    S::buffer(width, height, samples)
}

pub fn uncompressed_fixture<S: FixtureSample>(
    width: u32,
    height: u32,
    chunk_width: usize,
    chunk_height: usize,
    byte_order: ByteOrder,
    pattern: SyntheticPattern,
) -> SyntheticFixture {
    let image = synthetic_image::<S>(width, height, pattern);
    let encoded = EncodeCompression::Uncompressed
        .encode(&EncodeRequest {
            image: &image,
            chunk_width,
            chunk_height,
            byte_order,
            predictor: Predictor::None,
        })
        .unwrap();
    let mut source = Vec::new();
    let mut chunks = Vec::with_capacity(encoded.chunks.len());
    for bytes in encoded.chunks {
        let offset = source.len();
        source.extend_from_slice(&bytes);
        chunks.push(Chunk {
            offset,
            byte_count: bytes.len(),
        });
    }
    SyntheticFixture {
        image,
        source,
        chunks,
    }
}

fn pattern_word(pattern: SyntheticPattern, index: usize, width: usize) -> u64 {
    match pattern {
        SyntheticPattern::Smooth => {
            let x = index % width;
            let y = index / width;
            (x as u64)
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add((y as u64).wrapping_mul(0x85eb_ca6b))
        }
        SyntheticPattern::Noisy { seed } => {
            let mut value = seed.wrapping_add((index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        }
    }
}
