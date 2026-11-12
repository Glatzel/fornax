pub use fornax_codec::CodecError;

pub fn invalid<T>(detail: T) -> CodecError
where
    T: Into<Box<str>>,
{
    CodecError::InvalidInput(detail.into())
}
