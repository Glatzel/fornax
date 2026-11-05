use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;
use fornax::decoder::Decoder;
use fornax::postprocessor::PostProcessor;
use fornax::preprocessor::Preprocessor;
use fornax::protocol::IDecoder;
use fornax_core::RawFormat;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use fornax_decoder_dnc::{DncDecoder, DncParams};
use fornax_decoder_rawler::RawlerDecoder;
use fornax_devtool::save_image;
use image::{Luma, Rgb};
#[cfg(test)]
use strum::IntoEnumIterator;
use strum::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, EnumString, Default, Clone, AsRefStr, EnumIter)]
#[strum(ascii_case_insensitive)]
enum DecoderOption {
    #[default]
    Fornax,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
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

fn run<D, I>(decoder: D, arg: &Args) -> mischief::Result<()>
where
    D: IDecoder<I, f32>,
{
    let image = decoder.cropped_mosaic()?;
    let metadata = decoder.process_metadata()?;

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
    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    Ok(())
}

fn run_with(args: &Args) -> mischief::Result<()> {
    match args.decoder {
        DecoderOption::Fornax => run(Decoder::decode(&args.input)?, args),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        DecoderOption::Dnc => run(
            DncDecoder::decode(&(&args.input, &DncParams::default()))?,
            args,
        ),
        DecoderOption::Rawler => run(RawlerDecoder::decode(&args.input)?, args),
    }
}

fn main() -> mischief::Result<()> { run_with(&Args::parse()) }

#[test]
fn dng_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    fornax_devtool::setup();
    for decoder in DecoderOption::iter() {
        run_with(&Args {
            decoder,
            input: fornax_devtool::raw_file(RawFormat::Dng),
        })?;
    }
    Ok(())
}
