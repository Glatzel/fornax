use fornax_codec::{ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleFormat};

use crate::codec::IiqCodec;

const fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    _codec: IiqCodec,
    width: usize,
    height: usize,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        width,
        height,
        chunk_width: width,
        chunk_height: 1,
        chunks,
        bits_per_sample: 14,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    }
}

#[test]
fn iiq_uncompressed_scales_fourteen_bit_samples() {
    let source = [1, 0, 2, 0, 3, 0];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Uncompressed, 3, 1);

    let decoded = IiqCodec::Uncompressed.decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [4, 8, 12]);
}

#[test]
fn iiq_lossless_decodes_wide_rows_with_alternating_predictors() {
    // Two six-bit length headers select an eight-bit predictor. Each value is
    // then an eight-bit signed update biased by 127.
    let source = [2, 254, 253, 73, 2, 2, 2, 2, 0, 0, 0, 0];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Lossless, 8, 1);

    let decoded = IiqCodec::Lossless.decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [0, 0, 4, 4, 8, 8, 12, 12]);
}

#[test]
fn iiq_nonlinear_expands_low_values_through_the_s_curve() {
    let source = [1, 0, 0, 0, 0, 0, 2, 0];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Nonlinear, 3, 1);

    let decoded = IiqCodec::Nonlinear.decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [0, 0, 4]);
}

#[test]
fn iiq_sv2_decodes_a_partial_final_cluster() {
    let source = [4, 0, 0, 0, 192, 0, 32, 0];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Sv2, 3, 1);

    let decoded = IiqCodec::Sv2.decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [4, 8, 12]);
}

#[test]
fn iiq_rejects_a_chunk_count_that_does_not_match_rows() {
    let source = [0; 8];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Lossless, 8, 2);

    let error = IiqCodec::Lossless.decode(&request).unwrap_err().to_string();

    assert!(error.contains("chunk"), "{error}");
}

#[test]
fn iiq_rejects_non_integer_samples() {
    let source = [0; 8];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let mut request = request(&source, &chunks, IiqCodec::Uncompressed, 4, 1);
    request.sample_format = SampleFormat::Float;

    let error = IiqCodec::Uncompressed
        .decode(&request)
        .unwrap_err()
        .to_string();

    assert!(error.contains("integer"), "{error}");
}

#[test]
fn iiq_rejects_a_zero_lossless_code_length() {
    let source = [0, 0, 0, 0x80, 0, 0, 0, 0];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, IiqCodec::Lossless, 8, 1);

    let error = IiqCodec::Lossless.decode(&request).unwrap_err().to_string();

    assert!(error.contains("code length"), "{error}");
}
