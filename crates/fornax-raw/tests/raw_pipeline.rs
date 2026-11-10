use fornax_raw::postprocessor::Demosaic;
use fornax_raw::preprocessor::Preprocessor;

#[test]
fn facade_exposes_the_merged_pipeline_without_breaking_stage_paths() {
    let demosaic: fornax_raw::raw_pipeline::Demosaic = Demosaic::default();
    assert_eq!(demosaic, fornax_raw::raw_pipeline::Demosaic::default());

    let preprocessor: fornax_raw::raw_pipeline::Preprocessor = Preprocessor;
    let _: fornax_raw::Preprocessor = preprocessor;
}
