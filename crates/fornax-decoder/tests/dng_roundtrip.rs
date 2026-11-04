use std::fs;
use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use fornax_core::protocol::IDecoder;
use fornax_decoder::{Decoder, DngCompression, DngTag, DngValue, DngWriteOptions, MosaicData};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("external/raw-images/images/colorchart-5D2-6000K.dng")
}

#[test]
fn metadata_handoff_describes_the_full_bayer_mosaic() {
    let (decoded, metadata) =
        <Decoder as IDecoder<Path, f32>>::decode_full(&Decoder, &fixture_path())
            .expect("accepted DNG fixture should hand off to the Bayer pipeline");

    assert_eq!(decoded.image().dimensions(), (5_792, 3_804));
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

    let (active, active_metadata) =
        <Decoder as IDecoder<Path, f32>>::decode_active(&Decoder, &fixture_path())
            .expect("accepted DNG fixture should crop to its active area");
    assert_eq!(active.image().dimensions(), (5_634, 3_753));
    assert_eq!(active_metadata.bayer_pattern, BayerPattern::RGGB);
}

#[test]
fn uncompressed_export_round_trips_samples_and_required_metadata() {
    let decoded = Decoder
        .decode(&fixture_path())
        .expect("accepted DNG fixture should decode");
    let directory = tempfile::tempdir().expect("temporary output directory");
    let output_path = directory.path().join("uncompressed.dng");
    decoded
        .write_dng(
            &output_path,
            DngWriteOptions::new(DngCompression::Uncompressed),
        )
        .expect("uncompressed DNG export should succeed");

    let round_trip = Decoder
        .decode(&output_path)
        .expect("exported uncompressed DNG should decode");
    assert_eq!(round_trip.dimensions(), decoded.dimensions());
    assert_eq!(round_trip.pixels(), decoded.pixels());
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
            round_trip.metadata_by_tag(tag),
            decoded.metadata_by_tag(tag),
            "metadata tag {tag} was not preserved"
        );
    }
}

#[test]
fn default_lossless_jpeg_export_is_byte_identical_to_input() {
    let input_path = fixture_path();
    let input = fs::read(&input_path).expect("accepted DNG fixture should be readable");
    let decoded = Decoder
        .decode(&input_path)
        .expect("accepted DNG fixture should decode");

    assert_eq!(decoded.dimensions(), (5_792, 3_804));
    let MosaicData::U16(pixels) = decoded.pixels() else {
        panic!("the accepted fixture should decode to 16-bit samples");
    };
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    for &sample in pixels.as_raw() {
        sample_hash ^= u64::from(sample);
        sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    assert_eq!(sample_hash, 0x38b2_a452_5a2a_3049);
    assert_eq!(
        decoded.metadata_by_tag(DngTag::Make),
        Some(&DngValue::Ascii("Canon".into()))
    );
    assert_eq!(
        decoded.metadata_by_tag(DngTag::ImageWidth),
        Some(&DngValue::Unsigned(5_792))
    );

    let directory = tempfile::tempdir().expect("temporary output directory");
    let output_path = directory.path().join("roundtrip.dng");
    decoded
        .write_dng(&output_path, DngWriteOptions::default())
        .expect("default DNG export should succeed");

    let output = fs::read(output_path).expect("exported DNG should be readable");
    assert_eq!(output, input);
}
