use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::Parser;
use fornax::decoder::Decoder;
use fornax::postprocessor::PostProcessor;
use fornax::preprocessor::Preprocessor;
use fornax::protocol::IDecoder;
use fornax_core::RawFormat;
use fornax_devtool::save_image;
use image::{Luma, Rgb};
use strum::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, EnumString, Default, Clone, AsRefStr, EnumIter)]
#[strum(ascii_case_insensitive)]
enum DecoderOption {
    #[default]
    Fornax,
    Dnc,
    Rawler,
}

#[derive(Debug, Parser)]
#[command(about = "Run the RAW processing pipeline")]
struct Args {
    #[arg(long, short, value_enum, default_value=OsString::from(DecoderOption::default().as_ref()))]
    decoder: DecoderOption,
    /// Input RAW file (defaults to a DNG test file)
    #[arg(default_value =fornax_devtool::raw_file(RawFormat::Dng).into_os_string())]
    input: PathBuf,
}

fn run(arg: Args) -> mischief::Result<()> {
    let raw_path = arg.input;

    let (image, _, metadata) =
        <Decoder as IDecoder<&Path, f32>>::decode_active(&Decoder, raw_path.as_path())?;

    let output_dir = fornax_devtool::temp_dir()
        .join("pipeline")
        .join(arg.decoder.as_ref());

    let decoded_path = output_dir.join("1.decoded.tiff");
    save_image::<_, _, Luma<u16>, _>(image.image(), &decoded_path, 1.0)?;

    let processed = image.process(&Preprocessor, &metadata.levels)?;

    let preprocessed_path = output_dir.join("2.preprocess.tiff");
    save_image::<_, _, Luma<u16>, _>(processed.image(), &preprocessed_path, f32::from(u16::MAX))?;

    let demosaiced = processed.process(&PostProcessor::default(), &metadata)?;

    let demosaiced_path = output_dir.join("3.demosaic_linear.tiff");
    save_image::<_, _, Rgb<u16>, _>(&demosaiced, &demosaiced_path, f32::from(u16::MAX))?;
    dbg!(metadata.wb_coeffs);
    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());

    Ok(())
}

fn main() -> mischief::Result<()> { run(Args::parse()) }

#[test]
fn dng_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    fornax_devtool::setup();
    for d in RawFormat.iter() {
        run(Args {
            input: fornax_devtool::raw_file(RawFormat::Dng),
            decoder: d,
        });
    }
    Ok(())
}
