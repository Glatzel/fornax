# Rust TIFF and DNG Building Blocks

> **Task:** `RES-07`
> **Owner:** primary agent
> **Status:** `paused` (split evidence preserved; acceptance review deferred)
> **Last updated:** 2026-10-23
> **Upstream versions:** image-rs/image-tiff
> `3302936b08e2260d4c66f40f51170efbb1fded4c`; dnglab/rawler
> `62b77de1a206d467e910d55199191068b45a9cc1`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Bayer DNG capabilities and gaps are source-verified.
> - [x] Dependency use is compared with a minimal in-workspace parser.
> - [x] Sample layout, metadata access, limits, and errors are documented.
> - [x] License and maintenance evidence is recorded.
> - [x] Non-Bayer DNG payloads are excluded from initial support.
> - [x] The report contains the six required sections and cites sources.

## 1. Scope and questions

This report evaluates reusable Rust TIFF/DNG building blocks for the initial
Bayer DNG adapter. It does not repeat DNG tag semantics and defaults from
`RES-03` or the complete rawler decoder API from `RES-06`.

The questions were:

- Can a generic TIFF crate supply enough container and sample-layout support?
- Which DNG-specific pieces must remain in `fornax` or a dedicated adapter?
- What are the sample access, stride, error, limit, and compression boundaries?
- Is adding the dependency preferable to a minimal parser?

## 2. Findings

### 2.1 image-rs/image-tiff

`[verified]` image-tiff provides a typed generic TIFF `Decoder<R: Read + Seek>`,
image dimensions, `ColorType`, tag-keyed directories, sample formats, decoding
limits, row/plane stride preferences, strip/tile decoding, and structured
`TiffError`/format/unsupported/usage errors
(`src/decoder/mod.rs:15-100,246,861-900,1473-1500,1845-1945`;
`src/tags.rs:80-120,187-400`).

`[verified]` Its color model is generic TIFF/display imagery: gray, RGB, palette,
RGBA, CMYK, YCbCr, Lab, and multiband (`src/lib.rs:22-83`). It does not expose
RAW CFA semantics, active/default crop, black levels, or processing state. The
package is MIT licensed (`Cargo.toml`).

### 2.2 rawler's DNG-facing pieces

`[verified]` rawler demonstrates that a DNG adapter can retain CFAs, active and
crop areas, rational black levels, photometric interpretation, preserved tags,
and opcode metadata in a richer RAW type. The representation and its mutating
hazards are documented in `RES-06`; DNG tag semantics are documented in
`RES-03`.

`[inference]` A generic TIFF crate can reduce IFD, strip/tile, and typed tag
plumbing, but it does not replace a Bayer DNG adapter. DNG-specific
photometric handling, sample-domain interpretation, metadata validation, and
provenance remain `fornax` responsibilities.

### 2.3 Dependency versus minimal parser

| Option | Advantages | Costs and risks |
| --- | --- | --- |
| Depend on image-tiff | Maintained generic parsing, typed tags, strip/tile and limits support, MIT license | Still needs a DNG layer; must verify DNG photometric/compression/opcode coverage; new dependency surface |
| Depend on rawler | Rich DNG-facing behavior already present | LGPL-2.1; broad mutable RAW model and panics; couples more than the TIFF layer |
| Implement minimal parser | Maximum control over Bayer DNG slice | Owns TIFF/IFD, strips/tiles, compression hooks, limits, malformed-input handling, and compatibility risks |

`[recommendation]` Do not select a dependency solely because DNG is TIFF-based.
Defer the dependency decision to `RES-09` and design, after proving the required
Bayer DNG subset.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| image-tiff decoder, strides, limits, strip/tile access | image-tiff `src/decoder/mod.rs:15-100,246,861-900,1473-1500,1845-1945` |
| image-tiff tags and errors | image-tiff `src/tags.rs:80-120,187-400` |
| generic color model and no RAW metadata model | image-tiff `src/lib.rs:22-83` |
| image-tiff license | image-tiff `Cargo.toml` |
| rawler DNG representation | dnglab `rawler/src/rawimage.rs:203-276` |

## 4. Implications for fornax

`[recommendation]` Define the minimum Bayer DNG subset before choosing a parser:
required photometric values, sample formats, strip/tile encodings, compression,
CFA/crop/black/white tags, and rejection behavior.

`[recommendation]` Keep TIFF parsing behind the DNG adapter. Only the universal
RAW representation may cross into preprocessing and demosaicing.

`[recommendation]` Reject non-Bayer DNG payloads and unsupported DNG
physically-linear or opcode configurations with structured errors until a
separate task accepts them.

## 5. Open questions and limitations

- No end-to-end Bayer DNG fixture was decoded with image-tiff.
- Required DNG compression and opcode coverage was not experimentally measured.
- The cost of a minimal parser versus extending image-tiff remains quantitative
  work for design.
- rawler's full DNG error surface was not audited in this split.

## 6. References

- image-tiff: `https://github.com/image-rs/image-tiff.git` at
  `3302936b08e2260d4c66f40f51170efbb1fded4c`; MIT.
- rawler / dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`; LGPL-2.1.
