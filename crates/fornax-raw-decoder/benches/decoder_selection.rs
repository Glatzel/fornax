const ALL_DECODERS: &str = "fornax,rawler,flashraw";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderKind {
    Fornax,
    Rawler,
    FlashRaw,
}

pub fn parse_decoder_selection(value: Option<&str>) -> Result<Vec<DecoderKind>, String> {
    let value = value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(ALL_DECODERS);
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| match name {
            "fornax" => Ok(DecoderKind::Fornax),
            "rawler" => Ok(DecoderKind::Rawler),
            "flashraw" => Ok(DecoderKind::FlashRaw),
            _ => Err(format!(
                "unknown FORNAX_BENCH_DECODER value {name:?}; expected fornax, rawler, or flashraw"
            )),
        })
        .collect()
}

pub fn select_decoders(explicit: Option<&str>, ci: bool) -> Result<Vec<DecoderKind>, String> {
    let explicit = explicit.filter(|value| !value.trim().is_empty());
    parse_decoder_selection(explicit.or(ci.then_some("fornax")))
}
