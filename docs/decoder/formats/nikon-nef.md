# Nikon NEF Format

This page summarizes Nikon NEF mode support and rejection policy.

**Tier:** 1. **Status:** standard packed/uncompressed and 12/14-bit mode-3
lossless Huffman. Canonical detail:
[Nikon NEF reference notes](../../reference/decoders/reference/nikon-nef.md).

| Mode | Meaning | State |
| ---: | --- | --- |
| `1` | Lossy type-1 Huffman | Unsupported by policy |
| `2` | Uncompressed | Supported |
| `3` | Lossless Huffman, 12/14-bit, single strip | Supported |
| `4` | Lossy type-2 Huffman | Unsupported by policy |
| `5` | Striped packed 12-bit | Supported |
| `6` | Uncompressed reduced 12-bit | Supported |
| `7` | Unpacked 12-bit | Supported |
| `8` | Small/reduced RAW | Unsupported |
| `9` | Packed 12-bit | Supported |
| `10` | Packed/word 14-bit | Supported |
| `13`, `14` | High Efficiency / HE* | Unsupported |

Mode 3 uses the canonical 12- or 14-bit lossless tree, the supported `0x4630`
identity-curve metadata path, vertical then horizontal predictors, and exact
lossless output. Rawler's one-code identity-curve dither is reference-only.

Unsupported mode/layouts include lossy modes, small/HE/HE*, multi-strip mode 3,
multi-sample or reduced-linear data, Coolpix interlace, headerless maker notes,
and non-2x2 pipeline input.

## Acceptance

Every implemented mode needs passing corpus or synthetic coverage, exact
reference parity after crop/CFA/endianness/normalization alignment, typed
unsupported results, and tests for packing, row padding, Huffman prediction,
curve bounds, maker-note endianness, malformed streams, dispatch, and Bayer
handoff.
