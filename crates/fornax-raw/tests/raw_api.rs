use fornax_raw::dng::writer::{DngWriteError, DngWriter, DngWriterCompression};
use fornax_raw::{
    DecodeError, DemosaicError, PostProcessError, PreprocessError, RawError, RawValidationError,
};

#[test]
fn stage_errors_are_aliases_of_raw_error() {
    let errors: [RawError; 5] = [
        DecodeError::Unsupported("decode".into()),
        PreprocessError::EmptyImage,
        DemosaicError::InvalidInput("demosaic".into()),
        PostProcessError::InvalidColorTransform("color".into()),
        DngWriteError::WriteInvalidMetadata {
            detail: "writer".into(),
        },
    ];
    assert!(matches!(errors[4], RawError::WriteInvalidMetadata { .. }));
    let _: DecodeError = RawError::InvalidOutput(RawValidationError::InvalidImage);
}

#[test]
fn dng_writer_is_exposed_only_through_the_writer_module() {
    let _: DngWriter = DngWriter::new(DngWriterCompression::Uncompressed);
}
