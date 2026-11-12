use std::path::Path;

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;

pub enum ReferencePreflight<D> {
    Ready(D),
    Unsupported(Box<str>),
}

pub fn preflight_reference<D>(path: &Path) -> Result<ReferencePreflight<D>, DecodeError>
where
    D: IDecoder,
{
    let decoder = match D::decode(path) {
        Ok(decoder) => decoder,
        Err(DecodeError::Unsupported(detail)) => {
            return Ok(ReferencePreflight::Unsupported(detail));
        }
        Err(error) => return Err(error),
    };
    match decoder.raw_mosaic() {
        Ok(_) => Ok(ReferencePreflight::Ready(decoder)),
        Err(DecodeError::Unsupported(detail)) => Ok(ReferencePreflight::Unsupported(detail)),
        Err(error) => Err(error),
    }
}
