use std::path::Path;

use fornax_core::error::DecodeError as CoreDecodeError;
use fornax_core::metadata::Metadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{BayerPattern, DecodedRawActive, DecodedRawFull, Primitive};

use crate::{CfaPatternError, DecodedRaw, Decoder, HandoffError};

impl<T> IDecoder<Path, T> for Decoder
where
    T: Primitive,
{
    fn decode_full(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawFull<T>, Metadata<T>), CoreDecodeError> {
        Err(unsupported_core_decode())
    }

    fn decode_active(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), CoreDecodeError> {
        Err(unsupported_core_decode())
    }
}

impl DecodedRaw {
    /// Returns the Bayer pattern accepted by the preprocessor.
    ///
    /// # Errors
    ///
    /// Returns [`HandoffError::InvalidCfa`] for malformed CFA tags and
    /// [`HandoffError::UnsupportedCfa`] for valid but non-Bayer layouts.
    pub fn preprocessor_pattern(&self) -> Result<BayerPattern, HandoffError> {
        let repeat = self
            .cfa_repeat_pattern_dim()?
            .ok_or(CfaPatternError::MissingRepeatPatternDim)?;
        let pattern = self.cfa_pattern()?.ok_or(CfaPatternError::MissingPattern)?;
        let plane_colors = self.cfa_plane_colors()?.unwrap_or_else(|| vec![0, 1, 2]);
        let layout = self.cfa_layout()?.unwrap_or(1);

        if repeat != [2, 2] || plane_colors != [0, 1, 2] || layout != 1 {
            return Err(HandoffError::UnsupportedCfa {
                width: repeat[0],
                height: repeat[1],
            });
        }

        match pattern.as_slice() {
            [0, 1, 1, 2] => Ok(BayerPattern::RGGB),
            [2, 1, 1, 0] => Ok(BayerPattern::BGGR),
            [1, 0, 2, 1] => Ok(BayerPattern::GRBG),
            [1, 2, 0, 1] => Ok(BayerPattern::GBRG),
            _ => Err(HandoffError::UnsupportedCfa {
                width: repeat[0],
                height: repeat[1],
            }),
        }
    }
}

fn unsupported_core_decode() -> CoreDecodeError {
    CoreDecodeError::Unsupported("fornax-decoder scaffolding: RAW decoding is unsupported".into())
}
