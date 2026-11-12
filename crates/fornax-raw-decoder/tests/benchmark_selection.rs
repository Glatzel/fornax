#[path = "../benches/decoder_selection.rs"]
mod decoder_selection;

use decoder_selection::{DecoderKind, parse_decoder_selection};

#[test]
fn absent_and_empty_selections_use_all_benchmark_decoders() {
    let expected = vec![
        DecoderKind::Fornax,
        DecoderKind::Rawler,
        DecoderKind::FlashRaw,
    ];
    assert_eq!(parse_decoder_selection(None).unwrap(), expected);
    assert_eq!(parse_decoder_selection(Some("")).unwrap(), expected);
    assert_eq!(parse_decoder_selection(Some("  ")).unwrap(), expected);
}

#[test]
fn decoder_selection_preserves_requested_order() {
    assert_eq!(
        parse_decoder_selection(Some("flashraw, fornax")).unwrap(),
        vec![DecoderKind::FlashRaw, DecoderKind::Fornax]
    );
}

#[test]
fn decoder_selection_rejects_unknown_names() {
    let error = parse_decoder_selection(Some("fornax,unknown")).unwrap_err();
    assert!(error.contains("unknown"), "{error}");
}
