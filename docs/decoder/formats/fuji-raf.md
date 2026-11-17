# Fujifilm RAF Format

This page summarizes accepted RAF layouts and their pipeline boundary.

**Tier:** 2. **Status:** uncompressed and block-lossless images.
Canonical detail:
[Fujifilm RAF reference notes](../../reference/decoders/reference/fuji-raf.md).

## Supported

- Uncompressed 12-bit little-endian packed samples.
- Uncompressed 14/16-bit word samples.
- Fujifilm `IS`-signed block-lossless samples at 12, 14, and 16 bits.
- Bayer handoff for explicit GFX 2x2 profiles; X-Trans raw decode and DNG export
  preserve the 6x6 CFA but stop before preprocessing.

## Unsupported

Lossy block RAF, SuperCCD rotation/staggering, FinePix DBP, unknown Bayer
profiles, and non-2x2 pipeline handoff return typed unsupported errors. The
embedded JPEG is preview data, never a raw fallback.

## Acceptance

The corpus records supported Bayer and expected X-Trans/lossy/legacy outcomes.
X-Trans samples and 6x6 CFA must survive raw/DNG round-trip; GFX 2x2 files must
pass the Bayer handoff.
