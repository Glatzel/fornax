use fornax::decoder::{IDecoder, LibRawDecoder};
use fornax::{DecodedRaw, Demosaic};
use fornax_devtool::save_image;
use image::{Luma, Rgb};

fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::temp_dir().join("libraw-pipeline");
    let raw_path = fornax_devtool::raw_file();
    let decoded: DecodedRaw<f32> = LibRawDecoder::new().decode(raw_path.as_path())?;

    assert_eq!(decoded.image().dimensions(), (5_360, 3_516));
    assert_eq!(decoded.metadata().preprocess.active_area.width, 5_202);
    assert_eq!(decoded.metadata().preprocess.active_area.height, 3_464);
    let decoded_path = output_dir.join("1.fornax-libraw-decoded.tiff");
    dbg!(decoded.image().as_raw().get(0..10));
    save_image::<_, _, Luma<u16>, _>(decoded.image(), &decoded_path, 1.0)?;

    let (processed, metadata) = decoded.preprocess();
    assert_eq!(processed.image().dimensions(), (5_202, 3_464));
    let preprocessed_path = &output_dir.join("2.fornax-libraw-preprocess.tiff");
    save_image::<_, _, Luma<u16>, _>(processed.image(), &preprocessed_path, u16::MAX as _)?;

    let demosaiced = processed.demosaic(&Demosaic::Linear, metadata.preprocess.cfa.bayer_pattern);
    assert_eq!(demosaiced.dimensions(), (5_202, 3_464));
    let demosaiced_path = output_dir.join("3.fornax-libraw-demosaic_linear.tiff");
    save_image::<_, _, Rgb<u16>, _>(&demosaiced, &demosaiced_path, u16::MAX as _)?;

    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    println!(
        "{} {}",
        metadata.decoder.vendor.make.unwrap_or_default(),
        metadata.decoder.vendor.model.unwrap_or_default()
    );
    Ok(())
}

#[test]
fn libraw_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    main()?;
    for filename in [
        "1.fornax-libraw-decoded.tiff",
        "2.fornax-libraw-preprocess.tiff",
        "3.fornax-libraw-demosaic_linear.tiff",
    ] {
        assert!(
            fornax_devtool::temp_dir()
                .join("libraw-pipeline")
                .join(filename)
                .is_file()
        );
    }
    Ok(())
}
