use std::path::{Path, PathBuf};

use fornax_core::error::DngWriteError;
use fornax_core::metadata::dng::{DngTag, DngValue};
use fornax_core::protocol::IDecoder;
use fornax_core::writer::{DngCompression, DngWriter};
use fornax_core::{BayerPattern, Decoder, MosaicData};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("external/raw-images/images/colorchart-5D2-6000K.dng")
}

#[test]
fn accepted_dng_decodes_and_uncompressed_export_round_trips() {
    let input_path = fixture_path();
    let (decoded, metadata) =
        <Decoder as IDecoder<&Path, f32>>::decode(&Decoder, input_path.as_path())
            .expect("accepted DNG fixture should decode");

    assert_eq!(decoded.dimensions(), (5_792, 3_804));
    assert_eq!(
        (
            metadata.active_area.x,
            metadata.active_area.y,
            metadata.active_area.width,
            metadata.active_area.height,
        ),
        (158, 51, 5_634, 3_753)
    );
    assert_eq!(
        (
            metadata.default_crop.x,
            metadata.default_crop.y,
            metadata.default_crop.width,
            metadata.default_crop.height,
        ),
        (10, 5, 5_616, 3_744)
    );
    assert_eq!(metadata.bayer_pattern, BayerPattern::GBRG);
    assert_eq!(metadata.levels.black.dimensions(), (2, 2));
    assert!(
        metadata
            .levels
            .black
            .as_raw()
            .iter()
            .all(|&level| level == 1_024.0)
    );
    assert_eq!(metadata.levels.white, 15_600.0);

    let MosaicData::U16(pixels) = &decoded else {
        panic!("the accepted fixture should decode to 16-bit samples");
    };
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    for &sample in pixels.as_raw() {
        sample_hash ^= u64::from(sample);
        sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    assert_eq!(sample_hash, 0x38b2_a452_5a2a_3049);
    assert_eq!(
        metadata.dng_metadata.get(&DngTag::Make),
        Some(&DngValue::Ascii("Canon".into()))
    );
    assert_eq!(
        metadata.dng_metadata.get(&DngTag::ImageWidth),
        Some(&DngValue::Unsigned(5_792))
    );

    let (active, active_metadata) =
        <Decoder as IDecoder<&Path, f32>>::decode_active(&Decoder, input_path.as_path())
            .expect("accepted DNG fixture should crop to its active area");
    assert_eq!(active.image().dimensions(), (5_634, 3_753));
    assert_eq!(active_metadata.bayer_pattern, BayerPattern::RGGB);

    let directory = tempfile::tempdir().expect("temporary output directory");
    let output_path = directory.path().join("uncompressed.dng");
    DngWriter::new(DngCompression::Uncompressed)
        .write(&output_path, &decoded, &metadata.dng_metadata)
        .expect("uncompressed DNG export should succeed");

    let (round_trip, round_trip_metadata) =
        <Decoder as IDecoder<&Path, f32>>::decode(&Decoder, output_path.as_path())
            .expect("exported uncompressed DNG should decode");
    assert_eq!(round_trip, decoded);
    for tag in [
        DngTag::Make,
        DngTag::Model,
        DngTag::UniqueCameraModel,
        DngTag::ActiveArea,
        DngTag::DefaultCropOrigin,
        DngTag::DefaultCropSize,
        DngTag::CfaRepeatPatternDim,
        DngTag::CfaPattern,
        DngTag::BlackLevel,
        DngTag::WhiteLevel,
        DngTag::AsShotNeutral,
    ] {
        assert_eq!(
            round_trip_metadata.dng_metadata.get(&tag),
            metadata.dng_metadata.get(&tag),
            "metadata tag {tag} was not preserved"
        );
    }

    let lossless_path = directory.path().join("lossless.dng");
    let error = DngWriter::default()
        .write(&lossless_path, &decoded, &metadata.dng_metadata)
        .expect_err("default lossless export requires a source-preserving API");
    assert!(matches!(error, DngWriteError::Unsupported { .. }));
}
