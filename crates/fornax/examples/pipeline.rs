use std::path::PathBuf;

use fornax::metadata::Metadata;
use fornax::postprocessor::PostProcessor;
use fornax::preprocessor::Preprocessor;
use fornax::protocol::IDecoder;
use fornax::{DecodedRaw, Decoder};
use fornax_devtool::save_image;
use image::{Luma, Rgb};

fn run(input: Option<PathBuf>) -> mischief::Result<()> {
    let raw_path = input.unwrap_or_else(fornax_devtool::raw_file);

    let (image, metadata) = Decoder::default().decode_active(raw_path.as_path())?;
    let output_dir = fornax_devtool::temp_dir().join("pipeline");
    let decoded_path = output_dir.join("1.decoded.tiff");
    save_image::<_, _, Luma<u16>, _>(image.image(), &decoded_path, 1.0)?;
    let processed = image.process(&Preprocessor, &metadata.levels)?;
    let preprocessed_path = &output_dir.join("2.preprocess.tiff");
    save_image::<_, _, Luma<u16>, _>(processed.image(), preprocessed_path, u16::MAX as _)?;
    let demosaiced = processed.process(&PostProcessor::default(), &metadata)?;
    let demosaiced_path = output_dir.join("3.demosaic_linear.tiff");
    save_image::<_, _, Rgb<u16>, _>(&demosaiced, &demosaiced_path, u16::MAX as _)?;
    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    Ok(())
}
fn main() -> mischief::Result<()> {
    // optional single argument: path to a raw file
    run(std::env::args_os().nth(1).map(PathBuf::from))
}

#[test]
fn libraw_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    fornax_devtool::setup();
    run(None)
}
