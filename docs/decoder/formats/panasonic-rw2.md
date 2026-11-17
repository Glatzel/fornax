# Panasonic RW2 Format

This page summarizes accepted single-shot RW2 layouts and codec boundaries.

**Tier:** 2. **Status:** conventional 2x2 Bayer single-shot files.
Canonical detail:
[Panasonic RW2 reference notes](../../reference/decoders/reference/panasonic-rw2.md).

| Raw version | Compression | State |
| ---: | --- | --- |
| `4`, `5`, `6` | `34316` | Supported |
| `7` | any | Typed unsupported pending a real single-shot fixture |
| `8` | absent or `34316` | Supported |
| other | any | Typed unsupported |

Supported payloads include the V4/V5/V6/V8 families, V8 identity/nonlinear
tables, stamped 12-bit unpacked and packed-control layouts, multi-strip
placement, and 12/14/16-bit corpus fixtures. Output is `RawMosaic::U16` with
active-area process and DNG metadata.

Legacy `.RAW`, versions 1-3, V7, multi-exposure/pixel-shift/composite layouts,
unknown CFA, and non-2x2 pipeline input are unsupported. Checked geometry,
table sizes, strip coverage, and allocation bounds are required before output.

## Acceptance

Synthetic tests cover every codec layout and malformed/truncated streams.
Golden fixtures match Rawler storage samples. DNG round-trip covers active area,
crop, CFA, levels, neutral, and orientation. Manual corpus selection uses
`**/*.[rR][wW]2`; RAW payloads are never committed or uploaded.
