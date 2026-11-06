//! Support types shared by the decoder benchmark target.

/// A decoder implementation exposed through the shared `IDecoder` contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderKind {
    /// The default pure-Rust DNG decoder.
    Fornax,
    /// The Rawler adapter.
    Rawler,
}

/// Parses the decoder subset requested by `FORNAX_BENCH_DECODER`.
pub struct DecoderBenchSelection;

impl DecoderBenchSelection {
    /// Reads and parses the decoder benchmark selection from the environment.
    pub fn from_env() -> Result<Vec<DecoderKind>, String> {
        let value = std::env::var("FORNAX_BENCH_DECODER").ok();
        Self::from_value(value.as_deref())
    }

    /// Parses an optional comma-separated decoder list.
    ///
    /// `None` and an empty string select both decoders. Explicit values retain
    /// their order and duplicate names are ignored.
    pub fn from_value(value: Option<&str>) -> Result<Vec<DecoderKind>, String> {
        if value.is_none_or(|value| value.trim().is_empty()) {
            return Ok(vec![DecoderKind::Fornax, DecoderKind::Rawler]);
        }

        let mut selected = Vec::new();
        for name in value.unwrap().split(',').map(str::trim) {
            let decoder = match name {
                "fornax" => DecoderKind::Fornax,
                "rawler" => DecoderKind::Rawler,
                _ => {
                    return Err(format!(
                        "unknown FORNAX_BENCH_DECODER value {name:?}; expected fornax or rawler"
                    ));
                }
            };
            if !selected.contains(&decoder) {
                selected.push(decoder);
            }
        }
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_selection_defaults_to_both_and_preserves_explicit_order() {
        assert_eq!(
            DecoderBenchSelection::from_value(None).unwrap(),
            vec![DecoderKind::Fornax, DecoderKind::Rawler]
        );
        assert_eq!(
            DecoderBenchSelection::from_value(Some("rawler,fornax")).unwrap(),
            vec![DecoderKind::Rawler, DecoderKind::Fornax]
        );
        assert_eq!(
            DecoderBenchSelection::from_value(Some("rawler")).unwrap(),
            vec![DecoderKind::Rawler]
        );
    }

    #[test]
    fn decoder_selection_rejects_unknown_names() {
        assert_eq!(
            DecoderBenchSelection::from_value(Some("fornax,dnc")).unwrap_err(),
            "unknown FORNAX_BENCH_DECODER value \"dnc\"; expected fornax or rawler"
        );
    }
}
