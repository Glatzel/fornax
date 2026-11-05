mod dng_decoder;

use std::fs;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::DngMetadata;
use fornax_core::metadata::process::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};

#[derive(Clone, Debug)]
pub enum Decoder {
    Dng(dng_decoder::ParsedTiff,DngMetadata),
}
impl<P, T> IDecoder<P, T> for Decoder
where
    T: Primitive,
    P: AsRef<Path>,
{
    fn decode(input: P) -> Result<Self, DecodeError> {
        if let Some(extension) = input.as_ref().extension() {
            let decoder = match RawFormat::try_from(extension.to_str().unwrap()) {
                Ok(RawFormat::Dng) => {let decoded=dng_decoder::decode(&fs::read(input).unwrap())?;
                    let metadata=
                    Self::Dng(decoded);
                },
                _ => {
                    return Err(DecodeError::Unsupported(
                        format!("{}", extension.display()).into(),
                    ));
                }
            };
            Ok(decoder)
        } else {
            Err(DecodeError::Unsupported("Unknown extension.".into()))
        }
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        match self {
            Decoder::Dng(parsed_tiff) => ,
        }
    }

    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError> { todo!() }

    fn dng_metadata(&self) -> Result<DngMetadata, DecodeError> { todo!() }

    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError> { todo!() }
    // fn decode(
    //     &self,
    //     input: P,
    // ) -> Result<(RawMosaic, DngMetadata, ProcessMetadata<T>), DecodeError> {
    //     if let Some(extension) = input.as_ref().extension() {
    //         match RawFormat::try_from(extension.to_str().unwrap()) {
    //             Ok(RawFormat::Dng) =>
    // dng_decoder::decode(&fs::read(input).unwrap()),             _ =>
    // Err(DecodeError::Unsupported(                 format!("{}",
    // extension.display()).into(),             )),
    //         }
    //     } else {
    //         Err(DecodeError::Unsupported("Unknown extension.".into()))
    //     }
    // }

    // fn decode_active(
    //     &self,
    //     input: P,
    // ) -> Result<(CroppedMosaic<T>, DngMetadata, ProcessMetadata<T>),
    // DecodeError> {     if let Some(extension) = input.as_ref().extension() {
    //         match RawFormat::try_from(extension.to_str().unwrap()) {
    //             Ok(RawFormat::Dng) =>
    // dng_decoder::decode_active(&fs::read(input).unwrap()),             _ =>
    // Err(DecodeError::Unsupported(                 format!("{}",
    // extension.display()).into(),             )),
    //         }
    //     } else {
    //         Err(DecodeError::Unsupported("Unknown extension.".into()))
    //     }
    // }
}
