use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use fornax_decoder::Decoder;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::metadata::MetadataSource;
use fornax_raw_core::protocol::IDecoder;
use rawler::decoders::RawImageData;

#[test]
fn selected_nef_fixtures_match_rawler_raw_samples() {
    let Some(root) = fixture_root() else {
        return;
    };
    for relative in [
        "Nikon/D850/Nikon-D850-12bit-uncompressed.NEF",
        "Nikon/D850/Nikon-D850-14bit-uncompressed.NEF",
        "Nikon/D500/Nikon_D500_RAWL_12bit_Uncompressed.NEF",
        "Nikon/D850/Nikon-D850-12bit-lossless-compressed.NEF",
        "Nikon/Z 8/Nikon_Z8_raw_14_bit_lossless_compression.NEF",
    ] {
        let path = root.join(relative);
        if !is_materialized(&path) {
            continue;
        }
        let decoder = Decoder::decode(&path)
            .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
        let ours = match decoder.raw_mosaic().unwrap() {
            RawMosaic::U16(image) => image,
            other => panic!("{} decoded as {other:?}", path.display()),
        };
        let rawler = rawler::decode_file(&path)
            .unwrap_or_else(|error| panic!("Rawler failed on {}: {error}", path.display()));
        let reference = match rawler.data {
            RawImageData::Integer(samples) => samples,
            RawImageData::Float(_) => panic!("Rawler returned floating-point samples"),
        };
        assert_eq!(
            ours.dimensions(),
            (rawler.width as u32, rawler.height as u32),
            "{}",
            path.display()
        );
        let mismatch = ours
            .as_raw()
            .iter()
            .zip(&reference)
            .position(|(ours, reference)| !reference_matches(relative, *ours, *reference));
        if let Some(index) = mismatch {
            let differences = ours
                .as_raw()
                .iter()
                .zip(&reference)
                .filter(|(ours, reference)| ours != reference)
                .count();
            panic!(
                "{} differs at sample {index}: {} != {} ({differences} differences)",
                path.display(),
                ours.as_raw()[index],
                reference[index],
            );
        }

        let metadata = decoder.process_metadata().unwrap();
        assert!(metadata.active_area.width > 0);
        assert!(metadata.active_area.height > 0);
        assert!(metadata.levels.white > 0.0);
        assert!(
            !matches!(metadata.wb_source, MetadataSource::Unavailable { .. }),
            "{} WB source: {:?}",
            path.display(),
            metadata.wb_source
        );
        let cropped = decoder.cropped_mosaic::<f32>().unwrap();
        assert_eq!(
            cropped.image().dimensions(),
            (metadata.active_area.width, metadata.active_area.height)
        );
        assert!(decoder.dng_metadata().unwrap().len() > 10);
    }
}

fn fixture_root() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("FORNAX_RAW_SAMPLES")?);
    assert!(root.is_dir(), "FORNAX_RAW_SAMPLES is not a directory");
    Some(root)
}

fn is_materialized(path: &std::path::Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut header = [0; 64];
    let Ok(count) = file.read(&mut header) else {
        return false;
    };
    !header[..count].starts_with(b"version https://git-lfs.github.com/spec/v1")
}

fn reference_matches(path: &str, ours: u16, reference: u16) -> bool {
    if !path.contains("lossless-compressed") && !path.contains("lossless_compression") {
        return ours == reference;
    }
    // Rawler runs its identity curve through a dithering lookup, which emits
    // either the exact predictor or one code below it. Fornax keeps the
    // lossless predictor exact.
    ours == reference || reference.checked_add(1) == Some(ours)
}
