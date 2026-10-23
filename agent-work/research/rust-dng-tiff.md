# Rust TIFF and DNG Building Blocks

> **Task:** `RES-07`
> **Owner:** RES-07 research agent
> **Status:** `review` (ready for user review)
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
>
> **Review snapshot:** RES-07 research agent, 2026-10-23. The
> image-tiff public surface, sample/layout model, tag access, compression
> features, limits, errors, tests, and maintenance date were checked against
> the pinned revisions. DNG-specific behavior was separated from generic TIFF
> capability, and the dependency decision remains with `RES-09`. User review is
> pending.

## 1. Scope and questions

This report evaluates reusable Rust TIFF/DNG building blocks for the initial
Bayer DNG adapter. It does not repeat DNG tag semantics and defaults from
`RES-03` or the complete rawler decoder API from `RES-06`.

The questions were:

- Can a generic TIFF crate supply enough container and sample-layout support?
- Which DNG-specific pieces must remain in `fornax` or a dedicated adapter?
- What are the sample access, stride, error, limit, and compression boundaries?
- Is adding the dependency preferable to a minimal parser?

The target is a self-describing DNG containing a conventional 2x2 RGB Bayer
mosaic. LinearRaw, X-Trans, Foveon/X3, Quad Bayer, four-color, monochrome, and
other non-target DNG payloads are comparison evidence only and must be rejected
by the initial adapter.

## 2. Findings

### 2.1 image-rs/image-tiff

`[verified]` The inspected package is `tiff` 0.11.3, MIT licensed, at image-tiff
commit `3302936` dated 2026-09-06 (`Cargo.toml:2-10`; Git history). No published
long-term support policy was found, so recent source activity is the maintenance
evidence.

`[verified]` image-tiff exposes a typed generic TIFF `Decoder<R: Read + Seek>`,
image dimensions, `ColorType`, tag-keyed `Directory` values, sample formats,
decoding limits, caller-selectable row/plane strides, strip/tile decoding, and
structured format/unsupported/usage/I/O/limit errors
(`src/decoder/mod.rs:40-108,180-213,1544-1625,1845-1945`;
`src/directory.rs:22-45`; `src/error.rs:17-80`).

`[verified]` Its sample/layout API supports unsigned integer, signed integer,
IEEE floating-point, and void sample formats, with uniform bit depth per decoded
layout. The preferred layout reports sample type, row stride, plane count, and
plane stride; mixed per-channel sample formats are not represented
(`src/tags.rs:380-386`; `src/decoder/mod.rs:40-108`). Chunk output can therefore
preserve an explicit stride rather than forcing the current `fornax` contiguous
single-stride assumption.

`[verified]` Metadata is exposed as generic typed IFD entries. The directory
keeps known and unknown tag numbers, and the decoder provides `find_tag` /
`get_tag` and typed vector accessors (`src/directory.rs:22-45`;
`src/decoder/mod.rs:1988-2085`). This is sufficient plumbing to read DNG tags,
but it does not supply DNG defaults, coordinate conversion, CFA validation, or
processing state.

`[verified]` Compression support depends on compile features:

| Compression family | Support in inspected revision | Evidence |
| --- | --- | --- |
| Uncompressed | Always available | `src/decoder/image.rs:703` |
| PackBits | Built in | `src/decoder/image.rs:737` |
| LZW | Default feature | `Cargo.toml:41-46`; `src/decoder/image.rs:729-733` |
| Deflate / old Deflate | Default feature | `Cargo.toml:41-45`; `src/decoder/image.rs:739-742` |
| Modern JPEG (TIFF compression 7) | Default feature | `Cargo.toml:41-48`; `src/decoder/image.rs:743-794` |
| Zstandard | Optional feature | `Cargo.toml:48-51`; `src/decoder/image.rs:734-736` |
| WebP | Optional feature | `Cargo.toml:48-52`; `src/decoder/image.rs:810-817` |
| Other methods | `TiffUnsupportedError::UnsupportedCompressionMethod` | `src/decoder/image.rs:818-824` |

The table is generic TIFF capability, not proof that every DNG compression and
predictor combination required by the target corpus is supported. Those must be
verified with Bayer DNG fixtures before selecting this dependency.

`[verified]` Its color model is generic TIFF/display imagery: gray, RGB, palette,
RGBA, CMYK, YCbCr, Lab, and multiband (`src/lib.rs:22-83`). Its known tag set
contains TIFF/EXIF/GeoTIFF metadata but no dedicated CFA, ActiveArea,
DefaultCrop, black-level, white-level, or processing-state model
(`src/tags.rs:78-173`). Unknown DNG tags remain readable through generic tag
access, but semantic validation remains a `fornax` adapter responsibility.

`[verified]` Default limits cap a decoding result at 256 MiB, an intermediate
buffer at 128 MiB, and an IFD value at 1 MiB; the caller can configure them
(`src/decoder/mod.rs:180-213`). This is useful hardening for untrusted RAW input
but must be reconciled with full-resolution sensor buffers.

`[verified]` The checked-in tests cover generic strips/tiles, planar and
interleaved layouts, compression, floating-point samples, BigTIFF, and
subdirectories, but a source search of the test tree found no DNG, CFA, or RAW
metadata fixture (`tests/decode_libtiffpic.rs`, `tests/decode_planes.rs`,
`tests/decode_fp16_images.rs`, `tests/decode_bigtiff_images.rs`). Generic TIFF
coverage is therefore stronger than Bayer DNG coverage.

### 2.2 rawler's DNG-facing pieces

`[verified]` rawler demonstrates that a DNG adapter can retain CFAs, active and
crop areas, rational black levels, photometric interpretation, preserved tags,
and opcode metadata in a richer RAW type. The representation and its mutating
hazards are documented in `RES-06`; DNG tag semantics are documented in
`RES-03` (`rawler/src/rawimage.rs:203-276`).

`[verified]` The DNG decoder directly reads the raw IFD common tags, enforces
`PhotometricInterpretation` values 1, 32803, or 34892, extracts black/white
levels, CFA, active/default crop, orientation, and opcode lists, and delegates
sample decompression through its own TIFF/DNG path
(`rawler/src/decoders/dng.rs:53-145,287-309,380-515`).

`[verified]` rawler is version 0.8.0 and LGPL-2.1, with the inspected commit
dated 2026-10-23 (`rawler/Cargo.toml:8-13`; dnglab Git history). It is the most
complete DNG-facing Rust candidate but brings a broad mutable RAW model,
LGPL obligations, and decoder-specific behavior rather than a minimal TIFF
layer.

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

`[recommendation]` If image-tiff is selected, keep DNG semantics in a dedicated
`fornax` adapter and gate optional compression features explicitly. If rawler is
selected as a backend, adapt it outside `fornax-core` and snapshot metadata
before its mutating preprocessing calls.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| image-tiff decoder, layouts, strides, limits | image-tiff `src/decoder/mod.rs:40-108,180-213,1544-1625,1845-1945` |
| image-tiff IFD/tag access | image-tiff `src/directory.rs:22-45`; `src/decoder/mod.rs:1988-2085` |
| image-tiff compression features and error path | image-tiff `Cargo.toml:39-52`; `src/decoder/image.rs:702-824`; `src/error.rs:17-80` |
| image-tiff sample formats | image-tiff `src/tags.rs:380-386` |
| generic color model and absence of named DNG tags | image-tiff `src/lib.rs:22-83`; `src/tags.rs:78-173` |
| image-tiff license/version/revision date | image-tiff `Cargo.toml:2-10`; commit `3302936b`, 2026-09-06 |
| rawler DNG representation and decoder path | dnglab `rawler/src/rawimage.rs:203-276`; `rawler/src/decoders/dng.rs:53-145,287-309,380-515` |
| rawler license/version/revision date | dnglab `rawler/Cargo.toml:8-13`; commit `62b77de`, 2026-10-23 |

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
- Required DNG compression, predictor, opcode, and strip/tile coverage was not
  experimentally measured. Feature presence is not equivalent to target-format
  validation.
- The cost of a minimal parser versus extending image-tiff remains quantitative
  work for design.
- rawler's full DNG error, panic, and opcode surface was not audited in this
  split.
- No published maintenance/long-term support policy was found for either
  project; revision recency is the maintenance evidence.

## 6. References

- image-tiff: `https://github.com/image-rs/image-tiff.git` at
  `3302936b08e2260d4c66f40f51170efbb1fded4c`; MIT.
- rawler / dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`; LGPL-2.1.
