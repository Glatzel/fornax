use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DncError {
    #[error("DNC executable not found.")]
    ExecutableNotFound,
    #[error("Conversion failed.")]
    ConversionFailed,

    #[error("IoError.")]
    IoError(#[from] io::Error),
}
