use fornax_core::metadata::Levels;
use fornax_core::protocol::IPreprocessor;
use fornax_core::{BayerPattern, CroppedMosaic, MonoImage};
use fornax_raw_pipeline::{
    Demosaic, IDemosaic, LazyDemosaic, LmmseMode, OutputColorSpace, PostProcessor, Preprocessor,
};

#[test]
fn merged_pipeline_exposes_existing_stage_api() -> mischief::Result<()> {
    let cropped = CroppedMosaic::new(MonoImage::from_raw(4, 4, vec![0.5_f32; 16]).unwrap());
    let levels = Levels {
        black: MonoImage::from_raw(2, 2, vec![0.0; 4]).unwrap(),
        black_delta_h: Vec::new(),
        black_delta_v: Vec::new(),
        white: 1.0,
    };
    let normalized = Preprocessor.process(cropped, &levels)?;
    let rgb = Demosaic::Linear
        .demosaic(normalized.image(), BayerPattern::RGGB)?
        .collect();
    assert_eq!(rgb.dimensions(), (4, 4));

    let processor = PostProcessor::<f32, Demosaic>::default();
    assert_eq!(processor.output_color, OutputColorSpace::Raw);
    assert_eq!(LmmseMode::default(), LmmseMode::Median);
    Ok(())
}
