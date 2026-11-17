# Phase One IIQ Format

This page summarizes accepted IIQ modes, correction order, and acceptance.

**Tier:** 2. **Status:** Phase One/Leaf `.IIQ` modes 0/3/5/6/8. Canonical
detail:
[Phase One IIQ reference notes](../../reference/decoders/reference/phase-one-iiq.md).

| Mode | Layout | State |
| ---: | --- | --- |
| `0` | Uncompressed 14-bit words | Supported |
| `3` | IIQ-L row differences | Supported |
| `5` | IIQ-S nonlinear low values | Supported |
| `6` | IIQ-Sv2 clusters | Supported |
| `8` | IIQ-L16 row differences | Supported |
| `1`, `2` | Legacy Raw1/Raw2 | Typed unsupported |

`raw_mosaic` returns corrected U16 samples in this order: black and split-black,
polynomial curves, quadrant/combined gain and linearization, bad-column repair,
then flat fields. Corrected metadata exports black level zero and white level
65535.

Leaf `.MOS`, Phase One `.TIF`, preview extraction, legacy/unknown modes,
P65+ long-curve payloads, unknown camera profiles, and non-2x2 pipeline input
are typed unsupported.

## Acceptance

Modes 0/3/5/6/8 need synthetic or corpus coverage; compressed fixtures must
match Rawler after correction; unsupported variants must be typed; corrected
samples and CFA must survive DNG round-trip; facade routing, reviewed baseline,
and golden hashes must pass.
