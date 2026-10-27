use std::path::PathBuf;

use fornax::DecodedRawActive;
use fornax::decoder::{IDecoder, LibrawDecoder};
use fornax::metadata::Metadata;
use fornax_demosaic::Demosaic;
use fornax_devtool::save_image;
use fornax_preprocessor::Preprocessor;
use image::{Luma, Rgb};

fn wrapper(
    decoded: DecodedRawActive<f32>,
    metadata: Metadata<f32>,
    output_dir: PathBuf,
) -> mischief::Result<()> {
    assert_eq!(decoded.image().dimensions(), (5_202, 3_464));
    let decoded_path = output_dir.join("1.fornax-libraw-decoded.tiff");
    save_image::<_, _, Luma<u16>, _>(decoded.image(), &decoded_path, 1.0)?;

    let processed = decoded.preprocess(&Preprocessor, &metadata.levels)?;
    assert_eq!(processed.image().dimensions(), (5_202, 3_464));
    let preprocessed_path = &output_dir.join("2.fornax-libraw-preprocess.tiff");
    save_image::<_, _, Luma<u16>, _>(processed.image(), &preprocessed_path, u16::MAX as _)?;

    let demosaiced = processed.demosaic(&Demosaic::Linear, metadata.bayer_pattern)?;
    assert_eq!(demosaiced.dimensions(), (5_202, 3_464));
    let demosaiced_path = output_dir.join("3.fornax-libraw-demosaic_linear.tiff");
    save_image::<_, _, Rgb<u16>, _>(&demosaiced, &demosaiced_path, u16::MAX as _)?;

    println!(
        "Decoded output: {:?}",
        metadata.levels.black.as_raw().as_slice()
    );
    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    Ok(())
}
fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::temp_dir().join("pipeline");
    let raw_path = fornax_devtool::raw_file();
    //libraw
    {
        let (image, metadata) = LibrawDecoder.decode_active(raw_path.as_path())?;
        wrapper(image, metadata, output_dir.join("libraw"))?;
    }
    //dnc
    #[cfg(not(target_os = "linux"))]
    {
        let (image, metadata) =
            fornax::decoder::DncDecoder::default().decode_active(raw_path.as_path())?;
        wrapper(image, metadata, output_dir.join("dnc"))?;
    }
    //rawler
    {
        use fornax_decoder_rawler::RawlerDecoder;

        let (image, metadata) = RawlerDecoder.decode_active(raw_path.as_path())?;
        wrapper(image, metadata, output_dir.join("rawler"))?;
    }
    Ok(())
}

#[test]
fn libraw_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    main()?;
    Ok(())
}
