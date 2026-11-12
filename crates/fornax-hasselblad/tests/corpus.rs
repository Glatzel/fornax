use std::io::Cursor;
use std::path::PathBuf;

use fornax_dng::dng_decoder::DngImage;
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_hasselblad::HasselbladImage;
use fornax_raw_core::RawMosaic;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{ParsedTiff, Tag, Value, unsigned_scalar};
use rawler::decompressors::ljpeg::LjpegDecompressor;
use rawler::pixarray::PixU16;

const ACCEPTED_3FR: &[&str] = &[
    "Hasselblad/H3D/A9391120.3FR",
    "Hasselblad/X1D/B9999910.3FR",
    "Hasselblad/Hasselblad H4D/A0000902.3FR",
    "Hasselblad/X1D II 50C/MKW_20200429_0004569.3FR",
    "Hasselblad/cfv 50C II/B9979382.3FR",
    "Hasselblad/X2D 100C/B0000079.3FR",
    "Hasselblad/X2D 100C/B0000080.3FR",
    "Hasselblad/CFV-50c/20200215_BERLIN_OldNew_WestHafen_B0000420.3FR",
    "Hasselblad/CFV 100CElectronic Shutter/B0003499.3FR",
    "Hasselblad/CFV/RAW_HASSELBLAD_CFV.3FR",
    "Hasselblad/CF132/RAW_HASSELBLAD_IXPRESS_CF132.3FR",
];

#[test]
fn accepted_corpus_matches_rawler_and_hands_off() {
    let Some(root) = fixture_root() else {
        return;
    };
    for relative in ACCEPTED_3FR {
        let path = root.join(relative);
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            continue;
        }
        let decoder = HasselbladImage::parse(&source)
            .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
        let ours = match decoder.raw_mosaic().unwrap() {
            RawMosaic::U16(image) => image,
            other => panic!("3FR decoded as {other:?}"),
        };
        let (reference_width, reference_height, reference_samples) = reference_samples(&source);
        assert_eq!(
            ours.dimensions(),
            (reference_width as u32, reference_height as u32),
            "{relative} dimensions"
        );
        assert_eq!(
            ours.as_raw().len(),
            reference_samples.len(),
            "{relative} sample count"
        );
        if let Some((index, (ours, reference))) = ours
            .as_raw()
            .iter()
            .zip(reference_samples.iter())
            .enumerate()
            .find(|(_, (ours, reference))| ours != reference)
        {
            panic!("{relative} sample {index}: ours {ours}, Rawler {reference}");
        }

        let metadata = decoder.process_metadata().unwrap();
        let cropped = decoder.cropped_mosaic::<f32>().unwrap();
        assert_eq!(
            cropped.image().dimensions(),
            (metadata.active_area.width, metadata.active_area.height),
            "{relative} handoff"
        );
        assert!(decoder.dng_metadata().unwrap().len() > 20);
    }
}

#[test]
fn compressed_and_uncompressed_samples_round_trip_through_dng() {
    let Some(root) = fixture_root() else {
        return;
    };
    for relative in ["Hasselblad/H3D/A9391120.3FR", "Hasselblad/X1D/B9999910.3FR"] {
        let path = root.join(relative);
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            continue;
        }
        let decoder = HasselbladImage::parse(&source).unwrap();
        let RawMosaic::U16(ours) = decoder.raw_mosaic().unwrap() else {
            panic!("3FR decoded as a non-u16 mosaic");
        };
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::Uncompressed)
            .write_dng_with(
                &mut output,
                &RawMosaic::U16(ours.clone()),
                &decoder.dng_metadata().unwrap(),
            )
            .unwrap();
        let round_trip = DngImage::parse(output.get_ref()).unwrap();
        let RawMosaic::U16(round_trip) = round_trip.raw_mosaic().unwrap() else {
            panic!("round-tripped DNG decoded as a non-u16 mosaic");
        };
        assert_eq!(round_trip, ours, "{relative} round trip");
    }
}

fn fixture_root() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("FORNAX_RAW_SAMPLES")?);
    assert!(root.is_dir(), "FORNAX_RAW_SAMPLES is not a directory");
    Some(root)
}

fn reference_samples(source: &[u8]) -> (usize, usize, Vec<u16>) {
    let parsed = fornax_tiff::parse(source).unwrap();
    let raw = raw_directory(&parsed);
    let width = usize::try_from(unsigned_scalar(raw.get(&Tag::from(TiffTag::ImageWidth))).unwrap())
        .unwrap();
    let height =
        usize::try_from(unsigned_scalar(raw.get(&Tag::from(TiffTag::ImageLength))).unwrap())
            .unwrap();
    let offset =
        usize::try_from(unsigned_scalar(raw.get(&Tag::from(TiffTag::StripOffsets))).unwrap())
            .unwrap();
    let compression = unsigned_scalar(raw.get(&Tag::from(TiffTag::Compression))).unwrap() as u16;
    let samples = match compression {
        1 => source[offset..offset + width * height * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|word| u16::from_le_bytes(*word))
            .collect(),
        7 => {
            let decompressor = LjpegDecompressor::new_full(&source[offset..], true, false).unwrap();
            let mut output = PixU16::new(width, height);
            decompressor
                .decode(output.pixels_mut(), 0, width, width, height, false)
                .unwrap();
            output.pixels().to_vec()
        }
        other => panic!("unexpected reference compression {other}"),
    };
    (width, height, samples)
}

fn raw_directory(parsed: &ParsedTiff) -> &fornax_tiff::Directory {
    parsed
        .directories
        .iter()
        .filter(|directory| {
            matches!(
                directory.get(&Tag::from(TiffTag::NewSubfileType)),
                Some(Value::Unsigned(value)) if value & 1 == 0
            ) && directory.contains_key(&Tag::from(TiffTag::StripOffsets))
                && unsigned_scalar(directory.get(&Tag::from(TiffTag::Compression))).is_some()
                && unsigned_scalar(directory.get(&Tag::from(TiffTag::PhotometricInterpretation)))
                    == Some(32_803)
        })
        .max_by_key(|directory| {
            unsigned_scalar(directory.get(&Tag::from(TiffTag::ImageWidth))).unwrap_or(0)
                * unsigned_scalar(directory.get(&Tag::from(TiffTag::ImageLength))).unwrap_or(0)
        })
        .unwrap()
}
