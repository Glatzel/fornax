use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleBuffer, SampleFormat,
    decode,
};

fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    compression: DecodeCompression,
    width: usize,
    height: usize,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        compression,
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
    let request = request(&source, &chunks, DecodeCompression::IiqUncompressed, 3, 1);

    let decoded = decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [4, 8, 12]);
}

#[test]
fn iiq_lossless_decodes_wide_rows_with_alternating_predictors() {
    // Two six-bit length headers select an eight-bit predictor. Each value is
    // then an eight-bit signed update biased by 127.
    let source = [0x49, 0xfd, 0xfe, 0x02, 0x02, 0x02, 0x02, 0x02, 0x00];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, DecodeCompression::IiqLossless, 8, 1);

    let decoded = decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [0, 0, 4, 4, 8, 8, 12, 12]);
}

#[test]
fn iiq_nonlinear_expands_low_values_through_the_s_curve() {
    let source = [0, 0, 0, 1, 0, 2];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, DecodeCompression::IiqNonlinear, 3, 1);

    let decoded = decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [0, 0, 4]);
}

#[test]
fn iiq_sv2_decodes_a_partial_final_cluster() {
    let source = [0, 0, 0, 4, 0, 32, 0, 192];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, DecodeCompression::IiqSv2, 3, 1);

    let decoded = decode(&request).unwrap();

    assert_eq!(decoded.into_raw_u16().unwrap(), [4, 8, 12]);
}

#[test]
fn iiq_rejects_a_chunk_count_that_does_not_match_rows() {
    let source = [0; 8];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(&source, &chunks, DecodeCompression::IiqLossless, 8, 2);

    let error = decode(&request).unwrap_err().to_string();

    assert!(error.contains("chunk"), "{error}");
}

#[test]
fn iiq_rejects_non_integer_samples() {
    let source = [0; 8];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let mut request = request(&source, &chunks, DecodeCompression::IiqUncompressed, 4, 1);
    request.sample_format = SampleFormat::Float;

    let error = decode(&request).unwrap_err().to_string();

    assert!(error.contains("integer"), "{error}");
}
