#![cfg(feature = "raster")]

use std::io::Cursor;

use fornax_codec::{EncodeCompression, Predictor};
use fornax_core::{MonoImage, MultiPlaneImage, RgbImage, RgbaImage, UvImage};
use fornax_tiff::constants::PlanarConfiguration;
use fornax_tiff::image::{TiffImage, TypedTiffImage};
use fornax_tiff::parse;
use fornax_tiff::writer::{TiffWriteOptions, TiffWriter, WriteStorage};

fn assert_tiff_image_bit_eq(actual: &TiffImage, expected: &TiffImage, context: &str) {
    match (actual, expected) {
        (TiffImage::F32(a), TiffImage::F32(e)) => {
            assert_eq!(a.width(), e.width(), "{context}");
            assert_eq!(a.height(), e.height(), "{context}");
            assert_eq!(a.samples_per_pixel(), e.samples_per_pixel(), "{context}");
            assert_eq!(
                f32_bits(a),
                f32_bits(e),
                "f32 bit pattern mismatch for {context}"
            );
        }
        (TiffImage::F64(a), TiffImage::F64(e)) => {
            assert_eq!(a.width(), e.width(), "{context}");
            assert_eq!(a.height(), e.height(), "{context}");
            assert_eq!(a.samples_per_pixel(), e.samples_per_pixel(), "{context}");
            assert_eq!(
                f64_bits(a),
                f64_bits(e),
                "f64 bit pattern mismatch for {context}"
            );
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}

fn f32_bits(img: &TypedTiffImage<f32>) -> Vec<u32> {
    match img {
        TypedTiffImage::Mono(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Uv(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Rgb(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Rgba(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::MultiPlane(ps) => ps
            .iter()
            .flat_map(|p| p.as_raw().iter().map(|v| v.to_bits()))
            .collect(),
    }
}

fn f64_bits(img: &TypedTiffImage<f64>) -> Vec<u64> {
    match img {
        TypedTiffImage::Mono(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Uv(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Rgb(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::Rgba(b) => b.as_raw().iter().map(|v| v.to_bits()).collect(),
        TypedTiffImage::MultiPlane(ps) => ps
            .iter()
            .flat_map(|p| p.as_raw().iter().map(|v| v.to_bits()))
            .collect(),
    }
}

fn make_images_for_type<T: image::Primitive + Copy>(
    w: u32,
    h: u32,
    make: impl Fn(usize) -> T,
) -> Vec<(&'static str, TypedTiffImage<T>)> {
    let pixels = (w as usize) * (h as usize);
    let mono = MonoImage::from_raw(w, h, (0..pixels).map(&make).collect()).unwrap();
    let uv = UvImage::from_raw(w, h, (0..pixels * 2).map(&make).collect()).unwrap();
    let rgb = RgbImage::from_raw(w, h, (0..pixels * 3).map(&make).collect()).unwrap();
    let rgba = RgbaImage::from_raw(w, h, (0..pixels * 4).map(&make).collect()).unwrap();
    let multi: MultiPlaneImage<T> = (0..5)
        .map(|plane| {
            MonoImage::from_raw(
                w,
                h,
                (0..pixels).map(|i| make(i * 5 + plane)).collect::<Vec<_>>(),
            )
            .unwrap()
        })
        .collect();

    vec![
        ("mono", TypedTiffImage::Mono(mono)),
        ("uv", TypedTiffImage::Uv(uv)),
        ("rgb", TypedTiffImage::Rgb(rgb)),
        ("rgba", TypedTiffImage::Rgba(rgba)),
        ("multiplane5", TypedTiffImage::MultiPlane(multi)),
    ]
}

fn all_test_images(w: u32, h: u32) -> Vec<(String, TiffImage, bool)> {
    let mut out = Vec::new();
    for (name, img) in make_images_for_type(w, h, |i| ((i * 37 + 11) & 0xff) as u8) {
        out.push((format!("u8/{name}"), TiffImage::U8(img), false));
    }
    for (name, img) in make_images_for_type(w, h, |i| ((i * 977 + 0x1234) & 0xffff) as u16) {
        out.push((format!("u16/{name}"), TiffImage::U16(img), false));
    }
    for (name, img) in
        make_images_for_type(w, h, |i| (i as u32).wrapping_mul(0x9e37_79b9) ^ 0xdead_beef)
    {
        out.push((format!("u32/{name}"), TiffImage::U32(img), false));
    }
    for (name, img) in make_images_for_type(w, h, |i| (i as f32) * 0.125 - 3.5) {
        out.push((format!("f32/{name}"), TiffImage::F32(img), true));
    }
    for (name, img) in make_images_for_type(w, h, |i| (i as f64) * 0.0625 - 7.25) {
        out.push((format!("f64/{name}"), TiffImage::F64(img), true));
    }
    out
}

#[test]
fn full_matrix_round_trip_all_layouts_types_storages_and_codecs() {
    let width = 11_u32;
    let height = 7_u32;
    let images = all_test_images(width, height);

    let storages = [
        (
            "single_strip",
            WriteStorage::Strips {
                rows_per_strip: None,
            },
        ),
        (
            "partial_strips",
            WriteStorage::Strips {
                rows_per_strip: Some(3),
            },
        ),
        (
            "partial_tiles",
            WriteStorage::Tiles {
                width: 4,
                height: 4,
            },
        ),
    ];

    let planars = [
        ("chunky", PlanarConfiguration::Chunky),
        ("planar", PlanarConfiguration::Planar),
    ];

    let mut compressions = vec![
        ("uncompressed", EncodeCompression::Uncompressed),
        ("deflate", EncodeCompression::Deflate),
        ("lzw", EncodeCompression::Lzw),
        ("packbits", EncodeCompression::PackBits),
    ];
    #[cfg(feature = "zstd")]
    compressions.push(("zstd", EncodeCompression::Zstd { level: 1 }));

    for (img_label, image, is_float) in &images {
        for (storage_label, storage) in storages {
            for (planar_label, planar) in planars {
                for (comp_label, compression) in &compressions {
                    let predictors: &[(&str, Predictor)] =
                        if *compression == EncodeCompression::Uncompressed {
                            &[("none", Predictor::None)]
                        } else if *is_float {
                            &[
                                ("none", Predictor::None),
                                ("fp", Predictor::FloatingPoint { byte_stride: 1 }),
                            ]
                        } else {
                            &[
                                ("none", Predictor::None),
                                ("horiz", Predictor::HorizontalInteger),
                            ]
                        };

                    for (pred_label, predictor) in predictors {
                        let case = format!(
                            "{img_label}/{storage_label}/{planar_label}/{comp_label}/{pred_label}"
                        );
                        let options = TiffWriteOptions::default()
                            .with_compression(*compression)
                            .with_storage(storage)
                            .with_planar_configuration(planar)
                            .with_predictor(*predictor);

                        let mut buf = Cursor::new(Vec::new());
                        image
                            .write_to(&mut buf, &options)
                            .unwrap_or_else(|e| panic!("write failed for {case}: {e}"));

                        let decoded = TiffImage::from_bytes(buf.get_ref())
                            .unwrap_or_else(|e| panic!("read failed for {case}: {e}"));
                        assert_tiff_image_bit_eq(&decoded, image, &case);
                    }
                }
            }
        }
    }
}

#[test]
fn decode_planes_and_writer_write_image_work() {
    let rgb = RgbImage::from_raw(3, 2, (0_u8..18).collect()).unwrap();
    let image = TiffImage::from(rgb);
    let mut buf = Cursor::new(Vec::new());
    TiffWriter::new(&mut buf)
        .unwrap()
        .write_image(&image, &TiffWriteOptions::default())
        .unwrap();

    let parsed = parse(buf.get_ref()).unwrap();
    let planes_img = parsed.decode_planes(0).unwrap();
    let TiffImage::U8(TypedTiffImage::MultiPlane(planes)) = planes_img else {
        panic!("expected U8 MultiPlane");
    };
    assert_eq!(planes.len(), 3);
    assert_eq!(planes[0].as_raw(), &[0, 3, 6, 9, 12, 15]);
    assert_eq!(planes[1].as_raw(), &[1, 4, 7, 10, 13, 16]);
    assert_eq!(planes[2].as_raw(), &[2, 5, 8, 11, 14, 17]);
}
