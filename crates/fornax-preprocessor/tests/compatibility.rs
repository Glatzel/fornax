use fornax_core::metadata::Levels;
use fornax_core::protocol::IPreprocessor;
use fornax_core::{CroppedMosaic, MonoImage};
use fornax_preprocessor::Preprocessor;

#[test]
fn compatibility_path_normalizes_a_mosaic() -> mischief::Result<()> {
    let image = MonoImage::from_raw(2, 2, vec![0.25_f32; 4]).unwrap();
    let levels = Levels {
        black: MonoImage::from_raw(2, 2, vec![0.0; 4]).unwrap(),
        black_delta_h: Vec::new(),
        black_delta_v: Vec::new(),
        white: 1.0,
    };

    let output = Preprocessor.process(CroppedMosaic::new(image), &levels)?;
    assert_eq!(output.image().dimensions(), (2, 2));
    assert!(output.image().as_raw().iter().all(|value| *value == 0.25));
    Ok(())
}
