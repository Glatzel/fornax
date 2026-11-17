# Canon CR3 Format

This page summarizes the accepted Canon CR3/CRM scope and its completion gate.

**Tier:** 1. **Status:** audited lossless CRX only. Canonical detail:
[Canon CRX reference notes](../../reference/decoders/reference/canon-crx.md).

## Supported

- Canon CR3/CRM BMFF with the `crx` compatible brand and an explicit raw
  `CRAW` track/sample selection.
- CMP1 version `0x0100`, four planes, encoding type zero, zero wavelet levels,
  MDAT header size `0x70`, CFA layout `0..=3`, and bit depth `8..=16`.
- CMT/CTMD and IAD1 metadata mapping, checked crop conversion, U16 mosaic, and
  a conventional 2x2 Bayer handoff.

## Unsupported

C-RAW/lossy CRX, nonzero transform levels, ambiguous CMP1 tuples, preview/HEIF
and auxiliary dual-pixel images, CRM movie payloads as raw, non-2x2 pipeline
input, and lossy reconstruction all return typed unsupported errors.

## Acceptance

Targeted R5 and R6 Mark II mosaics must match Rawler digests exactly; C-RAW
must reject as unsupported; metadata, crop, levels, DNG output, and Bayer
handoff must be covered; the remote 110-file audit must classify every row.
