use std::path::Path;

#[path = "../src/decoder/libraw/native.rs"]
mod native;

use fornax::decoder::{IRawDecoder, LibRawDecoder};
use fornax::{BayerPattern, LumaImage, RgbImage, demosaic};
use image::{ImageBuffer, Luma, Rgb};
use native::{ColorData, ImageSizes, LibRawContext, UnpackedRawImageKind};

type Gray16 = ImageBuffer<Luma<u16>, Vec<u16>>;
type Rgb16 = ImageBuffer<Rgb<u16>, Vec<u16>>;

fn main() -> mischief::Result<()> {
    let libraw_dir = fornax_devtool::prepare_output_dir("compare-libraw", "libraw")?;
    let fornax_dir = fornax_devtool::prepare_output_dir("compare-libraw", "fornax")?;
    let raw_path = fornax_devtool::raw_file();

    println!("running decoded RAW comparison");
    compare_decoded(&libraw_dir, &fornax_dir, &raw_path)?;
    println!("running preprocessing comparison");
    compare_preprocessed(&libraw_dir, &fornax_dir, &raw_path)?;
    println!("running demosaic comparison");
    compare_demosaiced(&libraw_dir, &fornax_dir, &raw_path)?;
    Ok(())
}

fn compare_decoded(libraw_dir: &Path, fornax_dir: &Path, raw_path: &Path) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    let sizes = context.image_sizes();
    let color = context.color_data();
    let parameters = context.parameters()?;
    let pure = copy_integer_rows(&context)?;

    let decoded = LibRawDecoder::new().decode(raw_path)?;
    let metadata = decoded.metadata();
    assert_eq!(
        metadata.decoder.vendor.make.as_deref(),
        non_empty(&parameters.make)
    );
    assert_eq!(metadata.postprocess.white_balance.as_shot, color.cam_mul);
    assert_eq!(
        metadata.postprocess.color_matrices.camera_to_srgb,
        color.rgb_cam
    );
    assert!(!metadata.extra.is_empty());
    let fornax = raw_f32_plane_to_u16(decoded.image().full_image())?;

    pure.save(libraw_dir.join("decoded.tiff"))?;
    fornax.save(fornax_dir.join("decoded.tiff"))?;
    let diff = abs_diff_gray(&pure, &fornax)?;
    let stats = diff_stats(diff.as_raw());
    print_diff_stats("decoded", stats);
    assert!(
        stats.maximum == 0.0,
        "decoded RAW differs from LibRaw by {} codes",
        stats.maximum
    );
    assert_eq!(sizes.raw_width, pure.width());
    Ok(())
}

fn compare_preprocessed(
    libraw_dir: &Path,
    fornax_dir: &Path,
    raw_path: &Path,
) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    let sizes = context.image_sizes();
    let color = context.color_data();
    let parameters = context.parameters()?;
    let pure = normalized_reference(&context, &sizes, &color, parameters.filters)?;

    let (processed, metadata) = LibRawDecoder::new().decode(raw_path)?.preprocess()?;
    assert!(!metadata.decoder.backend.is_empty());
    let fornax = f32_plane_to_u16(processed.full_image())?;
    pure.save(libraw_dir.join("preprocess.tiff"))?;
    fornax.save(fornax_dir.join("preprocess.tiff"))?;
    let stats = diff_stats(abs_diff_gray(&pure, &fornax)?.as_raw());
    print_diff_stats("preprocess", stats);
    assert!(
        stats.maximum <= 1.0,
        "preprocessing differs from LibRaw by {} codes",
        stats.maximum
    );
    Ok(())
}

fn compare_demosaiced(
    libraw_dir: &Path,
    fornax_dir: &Path,
    raw_path: &Path,
) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    let sizes = context.image_sizes();
    let parameters = context.parameters()?;
    context.dcraw_process_neutral()?;
    let pure = context.make_processed_image()?.rgb16_image()?;

    let (processed, metadata) = LibRawDecoder::new().decode(raw_path)?.preprocess()?;
    let normalized = f32_plane_to_u16(processed.full_image())?;
    let mosaic = crop_gray(
        &normalized,
        sizes.left_margin,
        sizes.top_margin,
        sizes.width,
        sizes.height,
    )?;
    let tile = cfa_tile(parameters.filters)?;
    let scale = derive_libraw_scale(&pure, &mosaic, tile)?;
    let mosaic_view = processed.cropped(metadata.demosaic.active_area)?;
    let output = demosaic(mosaic_view, metadata.demosaic.bayer_pattern);
    let fornax = rgb_f32_to_u16(&output);

    pure.save(libraw_dir.join("demosaic_linear.tiff"))?;
    fornax.save(fornax_dir.join("demosaic_linear.tiff"))?;
    let stats = normalized_demosaic_stats(&pure, &fornax, &mosaic, scale);
    print_diff_stats("demosaic_linear", stats);
    assert!(
        stats.maximum <= 8.0,
        "demosaic maximum difference {} exceeds 8 codes",
        stats.maximum
    );
    assert!(
        stats.mean <= 2.0,
        "demosaic mean difference {} exceeds 2 codes",
        stats.mean
    );
    assert!(
        stats.rms <= 3.0,
        "demosaic RMS difference {} exceeds 3 codes",
        stats.rms
    );
    assert_eq!(
        metadata.demosaic.bayer_pattern,
        active_pattern(parameters.filters, &sizes)?
    );
    Ok(())
}

fn copy_integer_rows(context: &LibRawContext) -> mischief::Result<Gray16> {
    let raw = context.unpacked_raw_image()?;
    if raw.kind() != UnpackedRawImageKind::Integer {
        return Err(example_error("comparison fixture is not integer RAW"));
    }
    let mut samples = Vec::with_capacity(raw.raw_width() as usize * raw.raw_height() as usize);
    for row in 0..raw.raw_height() {
        samples.extend_from_slice(
            raw.integer_row(row)?
                .ok_or_else(|| example_error("LibRaw raw_image row is unavailable"))?,
        );
    }
    Gray16::from_raw(raw.raw_width(), raw.raw_height(), samples)
        .ok_or_else(|| example_error("LibRaw decoded buffer is inconsistent"))
}

fn normalized_reference(
    context: &LibRawContext,
    sizes: &ImageSizes,
    color: &ColorData,
    filters: u32,
) -> mischief::Result<Gray16> {
    let raw = context.unpacked_raw_image()?;
    if raw.kind() != UnpackedRawImageKind::Integer {
        return Err(example_error("comparison fixture is not integer RAW"));
    }
    let tile = cfa_tile(filters)?;
    let mut samples = Vec::with_capacity(
        usize::try_from(u64::from(sizes.raw_width) * u64::from(sizes.raw_height))
            .map_err(|_| example_error("RAW dimensions overflow"))?,
    );
    for row in 0..raw.raw_height() {
        let source = raw
            .integer_row(row)?
            .ok_or_else(|| example_error("LibRaw raw_image row is unavailable"))?;
        for (column, sample) in source.iter().enumerate() {
            let code = tile[((row & 1) * 2 + column as u32 % 2) as usize];
            let black = f64::from(color.black) + f64::from(color.cblack[code as usize]);
            let white = reference_white(color, code);
            let normalized = ((f64::from(*sample) - black) / (white - black)).clamp(0.0, 1.0);
            samples.push((normalized * f64::from(u16::MAX)).round() as u16);
        }
    }
    Gray16::from_raw(sizes.raw_width, sizes.raw_height, samples)
        .ok_or_else(|| example_error("normalized LibRaw buffer is inconsistent"))
}

fn reference_white(color: &ColorData, code: u8) -> f64 {
    if color.dng_whitelevel[0] != 0 {
        f64::from(color.dng_whitelevel[0])
    } else if color.linear_max.iter().all(|value| *value != 0) {
        f64::from(color.linear_max[code as usize])
    } else {
        f64::from(color.maximum)
    }
}

fn cfa_tile(filters: u32) -> mischief::Result<[u8; 4]> {
    let mut codes = [0; 4];
    for row in 0..2 {
        for column in 0..2 {
            let index = ((row & 7) * 2 + (column & 1)) * 2;
            codes[(row * 2 + column) as usize] = ((filters >> index) & 0b11) as u8;
        }
    }
    if !matches!(
        codes,
        [0, 1, 3, 2] | [2, 3, 1, 0] | [1, 0, 2, 3] | [3, 2, 0, 1]
    ) {
        return Err(example_error(
            "comparison fixture is not conventional Bayer",
        ));
    }
    Ok(codes)
}

fn active_pattern(filters: u32, sizes: &ImageSizes) -> mischief::Result<BayerPattern> {
    let codes = cfa_tile(filters)?;
    let top = sizes.top_margin & 1;
    let left = sizes.left_margin & 1;
    let code = |row: u32, column: u32| codes[((row & 1) * 2 + (column & 1)) as usize];
    match (
        code(top, left),
        code(top, left + 1),
        code(top + 1, left),
        code(top + 1, left + 1),
    ) {
        (0, 1, 3, 2) => Ok(BayerPattern::Rggb),
        (2, 3, 1, 0) => Ok(BayerPattern::Bggr),
        (1, 0, 2, 3) => Ok(BayerPattern::Grbg),
        (3, 2, 0, 1) => Ok(BayerPattern::Gbrg),
        _ => Err(example_error("active-area Bayer pattern is unsupported")),
    }
}

fn derive_libraw_scale(pure: &Rgb16, mosaic: &Gray16, tile: [u8; 4]) -> mischief::Result<f64> {
    let mut dot = 0.0;
    let mut square = 0.0;
    for (x, y, pixel) in mosaic.enumerate_pixels() {
        let value = f64::from(pixel[0]);
        if value == 0.0 || value >= 60_000.0 {
            continue;
        }
        let code = tile[((y & 1) * 2 + (x & 1)) as usize];
        let channel = match code {
            0 => 0,
            1 | 3 => 1,
            2 => 2,
            _ => return Err(example_error("invalid CFA code")),
        };
        let reference = f64::from(pure.get_pixel(x, y)[channel]);
        dot += reference * value;
        square += value * value;
    }
    if square == 0.0 {
        return Err(example_error("could not derive LibRaw output scale"));
    }
    Ok(dot / square)
}

fn normalized_demosaic_stats(pure: &Rgb16, fornax: &Rgb16, mosaic: &Gray16, scale: f64) -> Stats {
    let (width, height) = mosaic.dimensions();
    let mut differences = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if near_clipping(mosaic, x, y) {
                continue;
            }
            let expected = pure.get_pixel(x, y);
            let actual = fornax.get_pixel(x, y);
            for channel in 0..3 {
                let reference = (f64::from(expected[channel]) / scale)
                    .round()
                    .clamp(0.0, f64::from(u16::MAX)) as u16;
                differences.push(reference.abs_diff(actual[channel]));
            }
        }
    }
    diff_stats(&differences)
}

fn near_clipping(mosaic: &Gray16, x: u32, y: u32) -> bool {
    let (width, height) = mosaic.dimensions();
    for yy in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for xx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let value = mosaic.get_pixel(xx, yy)[0];
            if value == 0 || value >= 60_000 {
                return true;
            }
        }
    }
    false
}

fn crop_gray(
    source: &Gray16,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> mischief::Result<Gray16> {
    let mut samples = Vec::with_capacity(width as usize * height as usize);
    for y in top..top + height {
        for x in left..left + width {
            samples.push(source.get_pixel(x, y)[0]);
        }
    }
    Gray16::from_raw(width, height, samples)
        .ok_or_else(|| example_error("cropped grayscale buffer is inconsistent"))
}

fn f32_plane_to_u16(plane: &LumaImage) -> mischief::Result<Gray16> {
    let (width, height) = plane.dimensions();
    let samples = plane
        .as_raw()
        .iter()
        .map(|sample| (sample * f32::from(u16::MAX)).round() as u16)
        .collect();
    Gray16::from_raw(width, height, samples)
        .ok_or_else(|| example_error("f32 plane dimensions are inconsistent"))
}

fn raw_f32_plane_to_u16(plane: &LumaImage) -> mischief::Result<Gray16> {
    let (width, height) = plane.dimensions();
    let samples = plane
        .as_raw()
        .iter()
        .map(|sample| sample.round() as u16)
        .collect();
    Gray16::from_raw(width, height, samples)
        .ok_or_else(|| example_error("raw f32 plane dimensions are inconsistent"))
}

fn rgb_f32_to_u16(image: &RgbImage) -> Rgb16 {
    let samples = image
        .as_raw()
        .iter()
        .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
        .collect();
    Rgb16::from_raw(image.width(), image.height(), samples).expect("RGB dimensions are preserved")
}

fn abs_diff_gray(left: &Gray16, right: &Gray16) -> mischief::Result<Gray16> {
    if left.dimensions() != right.dimensions() {
        return Err(example_error(
            "cannot diff grayscale images with different dimensions",
        ));
    }
    let samples = left
        .as_raw()
        .iter()
        .zip(right.as_raw())
        .map(|(left, right)| left.abs_diff(*right))
        .collect();
    Gray16::from_raw(left.width(), left.height(), samples)
        .ok_or_else(|| example_error("grayscale diff dimensions are inconsistent"))
}

#[derive(Clone, Copy, Debug)]
struct Stats {
    maximum: f64,
    mean: f64,
    rms: f64,
    changed_percent: f64,
}

fn diff_stats(samples: &[u16]) -> Stats {
    let count = samples.len();
    let maximum = samples.iter().copied().max().unwrap_or_default();
    let sum: f64 = samples.iter().map(|sample| f64::from(*sample)).sum();
    let square_sum: f64 = samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum();
    let changed = samples.iter().filter(|sample| **sample != 0).count();
    Stats {
        maximum: f64::from(maximum),
        mean: if count == 0 { 0.0 } else { sum / count as f64 },
        rms: if count == 0 {
            0.0
        } else {
            (square_sum / count as f64).sqrt()
        },
        changed_percent: if count == 0 {
            0.0
        } else {
            changed as f64 * 100.0 / count as f64
        },
    }
}

fn print_diff_stats(stage: &str, stats: Stats) {
    println!(
        "{stage} abs diff: max={:.3}, mean={:.3}, rms={:.3}, changed={:.3}%",
        stats.maximum, stats.mean, stats.rms, stats.changed_percent
    );
}

fn example_error(message: &str) -> mischief::Report { mischief::mischief!("{message}") }

fn non_empty(value: &str) -> Option<&str> { (!value.is_empty()).then_some(value) }

#[test]
fn compare_libraw_example_writes_grouped_outputs() -> mischief::Result<()> {
    main()?;
    for producer in ["libraw", "fornax"] {
        for filename in ["decoded.tiff", "preprocess.tiff", "demosaic_linear.tiff"] {
            assert!(
                fornax_devtool::output_dir_for("compare-libraw", producer)
                    .join(filename)
                    .is_file()
            );
        }
    }
    Ok(())
}
