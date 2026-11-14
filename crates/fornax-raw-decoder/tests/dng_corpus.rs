#[path = "support/mosaic_compare.rs"]
mod mosaic_compare;
#[path = "../benches/rawler_wrapper.rs"]
mod rawler_wrapper;

use std::path::Path;

use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{BayerPattern, RawMosaic};
use fornax_raw_decoder::Decoder;
use mosaic_compare::{assert_mosaic_eq, canonical_mosaic};
use rawler_wrapper::RawlerDecoder;

#[test]
fn blackmagic_packed_row_lossless_matches_rawler() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    let path = Path::new(&root)
        .join("Blackmagic/Blackmagic Pocket Cinema Camera 4k/BMPCC_4k_DCI_lossless.dng");

    let decoder = Decoder::decode(&path).unwrap();
    let metadata = decoder.process_metadata().unwrap();
    assert_eq!(metadata.bayer_pattern, BayerPattern::RGGB);
    let fornax = decoder.raw_mosaic().unwrap();
    assert!(matches!(fornax, RawMosaic::U16(_)));
    assert_eq!(fornax.dimensions(), (4_128, 2_176));

    let rawler = RawlerDecoder::decode(&path).unwrap().raw_mosaic().unwrap();
    assert_mosaic_eq(
        "Blackmagic packed-row lossless",
        "rawler",
        &canonical_mosaic(&rawler),
        &canonical_mosaic(&fornax),
    );
}
