# RAW format support

These pages summarize current support, boundaries, and acceptance gates;
detailed source behavior lives in the linked decoder reference notes.

Use this directory to answer “does Fornax support this layout?” Use
`docs/reference/decoders/reference/` to understand container fields, codec
behavior, metadata mapping, provenance, and known numerical differences.

| Format | Support summary | Detailed reference |
| --- | --- | --- |
| Canon CR3/CRM | [Lossless CRX slice](canon-cr3.md) | [Canon CRX](../../reference/decoders/reference/canon-crx.md) |
| DNG | [Tier-0 baseline](dng.md) | [DNG decoder](../../reference/decoders/reference/dng-decoder.md) |
| Fujifilm RAF | [Lossless Bayer and X-Trans export](fuji-raf.md) | [Fujifilm RAF](../../reference/decoders/reference/fuji-raf.md) |
| Hasselblad 3FR | [Accepted Bayer profiles](hasselblad-3fr.md) | [Hasselblad 3FR](../../reference/decoders/reference/hasselblad-3fr.md) |
| Nikon NEF | [Modes 2/3/5/6/7/9/10](nikon-nef.md) | [Nikon NEF](../../reference/decoders/reference/nikon-nef.md) |
| Panasonic RW2 | [V4/V5/V6/V8 single-shot](panasonic-rw2.md) | [Panasonic RW2](../../reference/decoders/reference/panasonic-rw2.md) |
| Phase One IIQ | [Modes 0/3/5/6/8](phase-one-iiq.md) | [Phase One IIQ](../../reference/decoders/reference/phase-one-iiq.md) |
| Sony ARW/NRW | [Uncompressed, ARW1/2/6, LJPEG](sony-arw.md) | [Sony ARW/NRW](../../reference/decoders/reference/sony-arw.md) |
| Other RAW | [Explicit unsupported policy](other-formats.md) | |
