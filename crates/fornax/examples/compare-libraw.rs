use std::path::Path;

use fornax::decoder::libraw::native::{LibRawContext, UnpackedRawImage, UnpackedRawImageKind};
use fornax::decoder::{IRawDecoder, LibRawDecoder};
use fornax::{DemosaicAlgorithm, F32Plane, StoredPlane};
use image::{ImageBuffer, Luma, Rgb};

type Gray16 = ImageBuffer<Luma<u16>, Vec<u16>>;
type Rgb16 = ImageBuffer<Rgb<u16>, Vec<u16>>;

fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::temp_dir();
    std::fs::create_dir_all(&output_dir)?;
    let raw_path = fornax_devtool::raw_file();

    println!("running decoded comparison");
    compare_decoded(&output_dir, &raw_path)?;
    println!("running preprocess comparison");
    compare_preprocessed(&output_dir, &raw_path)?;
    println!("running demosaic comparison");
    compare_demosaiced(&output_dir, &raw_path)?;
    Ok(())
}

fn compare_decoded(output_dir: &Path, raw_path: &Path) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    let raw = context.unpacked_raw_image()?;
    let pure = match raw.kind() {
        UnpackedRawImageKind::Integer => copy_integer_rows(&raw)?,
        UnpackedRawImageKind::Float => {
            return Err(example_error(
                "comparison fixture unexpectedly uses a float raw_image",
            ));
        }
        UnpackedRawImageKind::None => {
            return Err(example_error("comparison fixture has no raw_image"));
        }
    };

    let fornax = LibRawDecoder::new().decode(raw_path)?;
    let fornax = u16_plane(&fornax)?;
    let pure_path = output_dir.join("libraw-decoded.tiff");
    let fornax_path = output_dir.join("fornax-libraw-decoded.tiff");
    pure.save(&pure_path)?;
    fornax.save(&fornax_path)?;
    let diff = abs_diff_gray(&pure, &fornax)?;
    diff.save(output_dir.join("libraw-vs-fornax-libraw-decoded-diff.tiff"))?;
    print_diff_stats("decoded", diff.as_raw());
    Ok(())
}

fn compare_preprocessed(output_dir: &Path, raw_path: &Path) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    let sizes = context.image_sizes();
    let color = context.color_data();
    let black = color.black;
    let cblack = color.cblack;
    let dng_white = color.dng_whitelevel[0];
    let linear_max = color.linear_max;
    let common_white = color.maximum;
    let rgba = context.active_rgba_image(true)?;
    let top = sizes.top_margin;
    let left = sizes.left_margin;
    let cfa_codes = [
        context.color(top as i32, left as i32),
        context.color(top as i32, (left + 1) as i32),
        context.color((top + 1) as i32, left as i32),
        context.color((top + 1) as i32, (left + 1) as i32),
    ];
    let has_per_color_white = cfa_codes
        .iter()
        .all(|code| linear_max[usize::try_from(*code).unwrap()] != 0);
    let mut pure_samples = Vec::with_capacity(rgba.width() as usize * rgba.height() as usize);
    for row in 0..rgba.height() {
        for column in 0..rgba.width() {
            let phase = ((row & 1) * 2 + (column & 1)) as usize;
            let color_index = usize::try_from(cfa_codes[phase])
                .map_err(|_| example_error("LibRaw returned a negative CFA color index"))?;
            let white = if dng_white != 0 {
                dng_white
            } else if has_per_color_white {
                linear_max[color_index]
            } else {
                common_white
            };
            let range = white.saturating_sub(black + cblack[color_index]);
            let value = rgba.get_pixel(column, row)[color_index];
            let normalized = if range == 0 {
                0
            } else {
                ((u64::from(value) * u64::from(u16::MAX) + u64::from(range / 2)) / u64::from(range))
                    .min(u64::from(u16::MAX)) as u16
            };
            pure_samples.push(normalized);
        }
    }
    let pure = Gray16::from_raw(rgba.width(), rgba.height(), pure_samples)
        .ok_or_else(|| example_error("pure LibRaw preprocessing buffer is inconsistent"))?;

    let fornax = LibRawDecoder::new()
        .decode(raw_path)?
        .preprocess()?
        .into_plane();
    let fornax = f32_plane_to_u16(&fornax)?;
    let pure_path = output_dir.join("libraw-preprocess.tiff");
    let fornax_path = output_dir.join("fornax-libraw-preprocess.tiff");
    pure.save(&pure_path)?;
    fornax.save(&fornax_path)?;
    let diff = abs_diff_gray(&pure, &fornax)?;
    diff.save(output_dir.join("libraw-vs-fornax-libraw-preprocess-diff.tiff"))?;
    print_diff_stats("preprocess", diff.as_raw());
    Ok(())
}

fn compare_demosaiced(output_dir: &Path, raw_path: &Path) -> mischief::Result<()> {
    let mut context = LibRawContext::new()?;
    context.open_file(raw_path)?;
    context.unpack()?;
    context.dcraw_process_neutral()?;
    let processed = context.make_processed_image()?;
    let pure = processed.rgb16_image()?;

    let fornax = LibRawDecoder::new()
        .decode(raw_path)?
        .preprocess()?
        .demosaic(DemosaicAlgorithm::Linear)?
        .0;
    let fornax = rgb_f32_to_u16(&fornax);
    let pure_path = output_dir.join("libraw-demosaic_linear.tiff");
    let fornax_path = output_dir.join("fornax-libraw-demosaic_linear.tiff");
    pure.save(&pure_path)?;
    fornax.save(&fornax_path)?;
    let diff = abs_diff_rgb(&pure, &fornax)?;
    diff.save(output_dir.join("libraw-vs-fornax-libraw-demosaic_linear-diff.tiff"))?;
    print_diff_stats("demosaic_linear", diff.as_raw());
    Ok(())
}

fn copy_integer_rows(raw: &UnpackedRawImage<'_>) -> mischief::Result<Gray16> {
    let width = raw.raw_width();
    let height = raw.raw_height();
    let mut samples = Vec::with_capacity(width as usize * height as usize);
    for row in 0..height {
        let source = raw
            .integer_row(row)?
            .ok_or_else(|| example_error("LibRaw raw_image row is unavailable"))?;
        samples.extend_from_slice(source);
    }
    Gray16::from_raw(width, height, samples)
        .ok_or_else(|| example_error("LibRaw decoded buffer is inconsistent"))
}

fn u16_plane(image: &fornax::DecodedRaw) -> mischief::Result<Gray16> {
    let StoredPlane::U16(plane) = image.plane() else {
        return Err(example_error(
            "fornax CR2 decode did not produce a u16 plane",
        ));
    };
    Ok(plane.clone())
}

fn f32_plane_to_u16(plane: &F32Plane) -> mischief::Result<Gray16> {
    let (width, height) = plane.dimensions();
    let samples = plane
        .as_raw()
        .iter()
        .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
        .collect();
    Gray16::from_raw(width, height, samples)
        .ok_or_else(|| example_error("f32 plane dimensions are inconsistent"))
}

fn rgb_f32_to_u16(image: &fornax::RgbImageF32) -> Rgb16 {
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

fn abs_diff_rgb(left: &Rgb16, right: &Rgb16) -> mischief::Result<Rgb16> {
    if left.dimensions() != right.dimensions() {
        return Err(example_error(
            "cannot diff RGB images with different dimensions",
        ));
    }
    let samples = left
        .as_raw()
        .iter()
        .zip(right.as_raw())
        .map(|(left, right)| left.abs_diff(*right))
        .collect();
    Rgb16::from_raw(left.width(), left.height(), samples)
        .ok_or_else(|| example_error("RGB diff dimensions are inconsistent"))
}

fn print_diff_stats(stage: &str, samples: &[u16]) {
    let count = samples.len();
    let maximum = samples.iter().copied().max().unwrap_or_default();
    let sum: f64 = samples.iter().map(|sample| f64::from(*sample)).sum();
    let square_sum: f64 = samples
        .iter()
        .map(|sample| {
            let value = f64::from(*sample);
            value * value
        })
        .sum();
    let changed = samples.iter().filter(|sample| **sample != 0).count();
    let mean = if count == 0 { 0.0 } else { sum / count as f64 };
    let rms = if count == 0 {
        0.0
    } else {
        (square_sum / count as f64).sqrt()
    };
    let changed_percent = if count == 0 {
        0.0
    } else {
        changed as f64 * 100.0 / count as f64
    };
    let full_scale = f64::from(u16::MAX);
    let max_percent = f64::from(maximum) * 100.0 / full_scale;
    let mean_percent = mean * 100.0 / full_scale;
    let rms_percent = rms * 100.0 / full_scale;
    println!(
        "{stage} abs diff: samples={count}, max={maximum}, mean={mean:.3}, rms={rms:.3}, \
         max_percent={max_percent:.3}%, mean_percent={mean_percent:.3}%, \
         rms_percent={rms_percent:.3}%, changed={changed} ({changed_percent:.3}%)"
    );
}

fn example_error(message: &str) -> mischief::Report { mischief::mischief!("{message}") }

#[test]
fn compare_libraw_example_writes_all_comparison_images() -> mischief::Result<()> {
    main()?;
    for filename in [
        "libraw-decoded.tiff",
        "fornax-libraw-decoded.tiff",
        "libraw-vs-fornax-libraw-decoded-diff.tiff",
        "libraw-preprocess.tiff",
        "fornax-libraw-preprocess.tiff",
        "libraw-vs-fornax-libraw-preprocess-diff.tiff",
        "libraw-demosaic_linear.tiff",
        "fornax-libraw-demosaic_linear.tiff",
        "libraw-vs-fornax-libraw-demosaic_linear-diff.tiff",
    ] {
        assert!(fornax_devtool::temp_dir().join(filename).is_file());
    }
    Ok(())
}
