use fornax::postprocessor::Demosaic;
use fornax::preprocessor::Preprocessor;

#[test]
fn facade_exposes_the_merged_pipeline_without_breaking_stage_paths() {
    let demosaic: fornax::raw_pipeline::Demosaic = Demosaic::default();
    assert_eq!(demosaic, fornax::raw_pipeline::Demosaic::default());

    let preprocessor: fornax::raw_pipeline::Preprocessor = Preprocessor;
    let _: fornax::Preprocessor = preprocessor;
}
