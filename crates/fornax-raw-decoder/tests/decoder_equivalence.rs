#[path = "../benches/decoder_inputs.rs"]
mod decoder_inputs;
#[path = "../benches/flashraw_wrapper.rs"]
mod flashraw_wrapper;
#[path = "support/mosaic_compare.rs"]
mod mosaic_compare;
#[path = "../benches/rawler_wrapper.rs"]
mod rawler_wrapper;

use decoder_inputs::benchmark_inputs;
use flashraw_wrapper::FlashRawDecoder;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;
use mosaic_compare::{assert_mosaic_eq, canonical_mosaic};
use rawler_wrapper::RawlerDecoder;

#[test]
#[ignore = "requires materialized decoder-tracker golden corpus"]
fn every_dng_benchmark_input_matches_across_decoders() {
    let inputs = benchmark_inputs();
    assert!(!inputs.is_empty(), "golden benchmark selection is empty");
    let mut checked = 0;

    for (input, path) in inputs {
        if input.format() != "DNG" {
            continue;
        }
        checked += 1;
        let case = input.case_label();
        let fornax = Decoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| {
                panic!(
                    "decoding Fornax mosaic for benchmark input {} ({case}) failed: {error}",
                    input.path,
                )
            });
        let rawler = RawlerDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| {
                panic!(
                    "decoding Rawler mosaic for benchmark input {} ({case}) failed: {error}",
                    input.path,
                )
            });
        let flashraw = FlashRawDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| {
                panic!(
                    "decoding FlashRaw mosaic for benchmark input {} ({case}) failed: {error}",
                    input.path,
                )
            });

        let fornax = canonical_mosaic(&fornax);
        assert_mosaic_eq(&case, "rawler", &canonical_mosaic(&rawler), &fornax);
        assert_mosaic_eq(&case, "flashraw", &canonical_mosaic(&flashraw), &fornax);
    }
    assert!(
        checked > 0,
        "golden selection contains no DNG benchmark input"
    );
}
