use fornax::decoder::{IDecoder, LibRawDecoder};
use fornax::{DecodedRaw, Demosaic};


fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::prepare_output_dir("libraw-pipeline")?;
    let raw_path = fornax_devtool::raw_file();
    let decoded: DecodedRaw<f32> = LibRawDecoder::new().decode(raw_path.as_path())?;

    assert_eq!(decoded.image().dimensions(), (5_360, 3_516));
    assert_eq!(decoded.metadata().preprocess.active_area.width, 5_202);
    assert_eq!(decoded.metadata().preprocess.active_area.height, 3_464);
    let decoded_path = output_dir.join("fornax-libraw-decoded.tiff");
    decoded.image().save(&decoded_path)?;

    let (processed, metadata) = decoded.preprocess();
    assert_eq!(processed.image().dimensions(), (5_202, 3_464));
    let preprocessed_path = output_dir.join("fornax-libraw-preprocess.tiff");
    processed.image().save(&preprocessed_path)?;

    let demosaiced = processed.demosaic(&Demosaic::Linear, metadata.preprocess.cfa.bayer_pattern);
    assert_eq!(demosaiced.dimensions(), (5_202, 3_464));
    let demosaiced_path = output_dir.join("fornax-libraw-demosaic_linear.tiff");
    demosaiced.save(&demosaiced_path)?;

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
        "fornax-libraw-decoded.tiff",
        "fornax-libraw-preprocess.tiff",
        "fornax-libraw-demosaic_linear.tiff",
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
