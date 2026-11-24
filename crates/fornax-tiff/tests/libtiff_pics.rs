#![cfg(feature = "raster")]

use std::io::Cursor;
use std::path::{Path, PathBuf};

use fornax_codec::EncodeCompression;
use fornax_tiff::constants::{PlanarConfiguration, TiffTag};
use fornax_tiff::{
    TiffImage, TiffIoError, TiffWriteOptions, TypedTiffImage, WriteStorage, parse, read_tiff,
    read_tiff_file, unsigned_scalar, write_tiff,
};
use image::DynamicImage;

fn collect_tiff_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read libtiff-pics directory") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }
                stack.push(path);
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
                })
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn oracle_matches(path: &Path, actual: &TiffImage) -> Option<String> {
    let oracle = image::open(path).ok()?;
    let (ow, oh) = (oracle.width(), oracle.height());
    if (actual.width(), actual.height()) != (ow, oh) {
        return Some(format!(
            "dimension mismatch: fornax={}x{}, oracle={}x{}",
            actual.width(),
            actual.height(),
            ow,
            oh
        ));
    }
    match (actual, oracle) {
        (TiffImage::U8(TypedTiffImage::Mono(img)), DynamicImage::ImageLuma8(o)) => {
            let bytes = std::fs::read(path).ok()?;
            let parsed = parse(&bytes).ok()?;
            let bits =
                unsigned_scalar(parsed.root().get(&TiffTag::BitsPerSample.into())).unwrap_or(8);
            if bits < 8 {
                let max_in = ((1_u32 << bits) - 1).max(1);
                let scaled: Vec<u8> = img
                    .as_raw()
                    .iter()
                    .map(|&v| ((u32::from(v) * 255 + max_in / 2) / max_in) as u8)
                    .collect();
                if &scaled != o.as_raw() {
                    return Some(format!("u8 ({bits}-bit) mono pixel mismatch"));
                }
            } else if img.as_raw() != o.as_raw() {
                return Some("u8 mono pixel mismatch".into());
            }
        }
        (TiffImage::U8(TypedTiffImage::Uv(img)), DynamicImage::ImageLumaA8(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u8 uv pixel mismatch".into());
            }
        }
        (TiffImage::U8(TypedTiffImage::Rgb(img)), DynamicImage::ImageRgb8(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u8 rgb pixel mismatch".into());
            }
        }
        (TiffImage::U8(TypedTiffImage::Rgba(img)), DynamicImage::ImageRgba8(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u8 rgba pixel mismatch".into());
            }
        }
        (TiffImage::U16(TypedTiffImage::Mono(img)), DynamicImage::ImageLuma16(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u16 mono pixel mismatch".into());
            }
        }
        (TiffImage::U16(TypedTiffImage::Rgb(img)), DynamicImage::ImageRgb16(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u16 rgb pixel mismatch".into());
            }
        }
        (TiffImage::U16(TypedTiffImage::Rgba(img)), DynamicImage::ImageRgba16(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("u16 rgba pixel mismatch".into());
            }
        }
        (TiffImage::F32(TypedTiffImage::Rgb(img)), DynamicImage::ImageRgb32F(o)) => {
            if img.as_raw() != o.as_raw() {
                return Some("f32 rgb pixel mismatch".into());
            }
        }
        (TiffImage::F32(TypedTiffImage::Rgba(img)), DynamicImage::ImageRgba32F(o))
            if img.as_raw() != o.as_raw() =>
        {
            return Some("f32 rgba pixel mismatch".into());
        }
        _ => {}
    }
    None
}

#[test]
fn libtiff_pics_corpus_decode_oracle_and_roundtrip() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external/libtiff-pics");
    assert!(
        corpus_dir.is_dir(),
        "external/libtiff-pics submodule is missing at {}",
        corpus_dir.display()
    );

    let files = collect_tiff_files(&corpus_dir);
    assert!(
        files.len() >= 50,
        "expected at least 50 TIFF files in libtiff-pics, found {}",
        files.len()
    );

    let mut decoded_count = 0_usize;
    let mut oracle_checked = 0_usize;
    let mut oracle_mismatches = Vec::new();
    let mut roundtrip_count = 0_usize;
    let mut unsupported_count = 0_usize;
    let mut unexpected_errors = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(&corpus_dir)
            .unwrap_or(path)
            .display()
            .to_string();
        let meta = std::fs::metadata(path).expect("file metadata");
        assert!(
            meta.len() > 256,
            "{rel} is only {} bytes; Git LFS checkout may be missing",
            meta.len()
        );

        match read_tiff_file(path) {
            Ok(img) => {
                decoded_count += 1;
                assert!(img.width() > 0 && img.height() > 0, "{rel}");
                assert!(img.samples_per_pixel() >= 1, "{rel}");

                if let Some(mismatch) = oracle_matches(path, &img) {
                    oracle_mismatches.push(format!("{rel}: {mismatch}"));
                } else if image::open(path).is_ok() {
                    oracle_checked += 1;
                }

                // Round-trip through both Strips+Chunky+Deflate and
                // Tiles+Planar+Lzw.
                for options in [
                    TiffWriteOptions::default()
                        .with_compression(EncodeCompression::Deflate)
                        .with_storage(WriteStorage::Strips {
                            rows_per_strip: Some(16),
                        })
                        .with_planar_configuration(PlanarConfiguration::Chunky),
                    TiffWriteOptions::default()
                        .with_compression(EncodeCompression::Lzw)
                        .with_storage(WriteStorage::Tiles {
                            width: 32,
                            height: 32,
                        })
                        .with_planar_configuration(PlanarConfiguration::Planar),
                ] {
                    let mut buf = Cursor::new(Vec::new());
                    write_tiff(&mut buf, &img, &options)
                        .unwrap_or_else(|e| panic!("roundtrip write failed for {rel}: {e}"));
                    let redecoded = read_tiff(buf.get_ref())
                        .unwrap_or_else(|e| panic!("roundtrip read failed for {rel}: {e}"));
                    assert_eq!(redecoded, img, "roundtrip mismatch for {rel}");
                }
                roundtrip_count += 1;
            }
            Err(
                TiffIoError::Unsupported(_) | TiffIoError::Compression(_) | TiffIoError::Codec(_),
            ) => {
                unsupported_count += 1;
            }
            Err(other) => {
                unexpected_errors.push(format!("{rel}: {other}"));
            }
        }
    }

    eprintln!(
        "libtiff-pics summary: total={}, decoded={}, oracle_checked={}, roundtripped={}, unsupported={}",
        files.len(),
        decoded_count,
        oracle_checked,
        roundtrip_count,
        unsupported_count
    );

    assert!(
        unexpected_errors.is_empty(),
        "unexpected errors on libtiff-pics:\n{}",
        unexpected_errors.join("\n")
    );
    assert!(
        oracle_mismatches.is_empty(),
        "oracle mismatches on libtiff-pics:\n{}",
        oracle_mismatches.join("\n")
    );
    assert!(
        decoded_count >= 20,
        "expected at least 20 decoded libtiff-pics files, got {decoded_count}"
    );
}
