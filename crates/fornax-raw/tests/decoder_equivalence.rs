#[path = "../benches/decoder_inputs.rs"]
mod decoder_inputs;
#[path = "../benches/flashraw_wrapper.rs"]
mod flashraw_wrapper;
#[path = "support/mosaic_compare.rs"]
mod mosaic_compare;
#[path = "../benches/rawler_wrapper.rs"]
mod rawler_wrapper;

use decoder_inputs::{benchmark_inputs, test_inputs};
use flashraw_wrapper::FlashRawDecoder;
use fornax_raw::Decoder;
use fornax_raw::protocol::IDecoder;
use mosaic_compare::{assert_mosaic_eq, canonical_mosaic};
use rawler_wrapper::RawlerDecoder;

#[test]
#[ignore = "requires materialized decoder-tracker golden corpus"]
fn every_dng_benchmark_input_matches_across_decoders() {
    let inputs = test_inputs();
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

#[test]
fn known_correctness_regressions_match_rawler_and_flashraw_when_available() {
    let corpus =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external/raw.pixls.us");
    for path in ["Sony/ILCE-7RM4/DSC00395.ARW", "Pentax/K-1/IMGP8555.DNG"] {
        let path = corpus.join(path);
        if !path.is_file() {
            continue;
        }
        let case = path.file_name().unwrap().to_string_lossy();
        let fornax = Decoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("Fornax failed for {case}: {error}"));
        let rawler = RawlerDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("Rawler failed for {case}: {error}"));
        let flashraw = FlashRawDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("FlashRaw failed for {case}: {error}"));
        let fornax = canonical_mosaic(&fornax);
        assert_mosaic_eq(&case, "rawler", &canonical_mosaic(&rawler), &fornax);
        assert_mosaic_eq(&case, "flashraw", &canonical_mosaic(&flashraw), &fornax);
    }
}

#[test]
fn configured_manifest_inputs_match_rawler_and_flashraw_when_requested() {
    if std::env::var_os("FORNAX_CROSSCHECK_MANIFEST").is_none() {
        return;
    }
    for (input, path) in benchmark_inputs() {
        let case = input.case_label();
        let fornax = Decoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("Fornax failed for {case}: {error}"));
        let rawler = RawlerDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("Rawler failed for {case}: {error}"));
        let flashraw = FlashRawDecoder::decode(&path)
            .and_then(|decoder| decoder.raw_mosaic())
            .unwrap_or_else(|error| panic!("FlashRaw failed for {case}: {error}"));
        let fornax = canonical_mosaic(&fornax);
        assert_mosaic_eq(&case, "rawler", &canonical_mosaic(&rawler), &fornax);
        assert_mosaic_eq(&case, "flashraw", &canonical_mosaic(&flashraw), &fornax);
    }
}
