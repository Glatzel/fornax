# dnglab and DNG Metadata

> **Task:** `RES-03`
> **Owner:** RES-03 research agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-23
> **Upstream versions:** dnglab `62b77de1a206d467e910d55199191068b45a9cc1`
> (including its `rawler/` subtree); Adobe DNG SDK mirror
> `962a7028efdc8f8a651cf6335c3ec53053472339`, containing DNG Specification
> 1.7.0.0 dated June 2023.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] DNG tags relevant to sample interpretation and preprocessing are mapped
>   to source symbols, with units, defaults, and coordinate conventions.
> - [x] `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `CFAPattern`,
>   `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`, `Orientation`, and
>   related tags are covered or explicitly excluded with a reason.
> - [x] Differences between DNG and vendor RAW metadata are identified.
> - [x] Licensing constraints are recorded.
> - [x] Standard Bayer DNG is distinguished from deferred non-Bayer DNG
>   payloads.
> - [x] The report contains the six required sections, evidence labels, and
>   cited source locations.
>
> **Verification snapshot:** RES-03 research agent and primary checks completed
> 2026-10-23; GitHub pull-request review is pending. DNG tag mappings, specification defaults,
> ActiveArea/DefaultCrop/CFA coordinate rules, rawler limitations,
> vendor-metadata distinctions, and licensing were checked against the pinned
> sources. Stale references to a missing extracted text file were replaced with
> stable PDF page/tag references, and the `DefaultCropSize` fallback was
> corrected to ActiveArea dimensions using Adobe SDK source.

## 1. Scope and questions

This report examines how dnglab/rawler decodes a DNG raw image, how it represents
the stored samples and metadata, and how the relevant DNG 1.7.0.0 tags define
sample interpretation, sensor coordinates, and black/white-level processing.

The questions were:

- What does the decoded DNG buffer contain before and after rawler's scaling?
- Which DNG tags identify the raw sample encoding, CFA, active area, default
  crop, black levels, white levels, and orientation?
- What are each tag's type, count, default, unit, and coordinate basis?
- How do stored-image, ActiveArea, and DefaultCrop coordinates relate?
- How are repeating black-level patterns and CFA phase aligned after cropping?
- Which DNG semantics are preserved, normalized, or narrowed by rawler?
- How do self-describing DNG metadata and vendor RAW metadata differ?
- What licensing constraints apply to reuse?

Only dnglab/rawler and Adobe's published specification are used for the DNG
claims. LibRaw behavior is not used to fill gaps in DNG semantics.

The initial implementation target is a DNG payload containing a conventional
2x2 RGB Bayer mosaic.

| Payload class | Treatment |
| --- | --- |
| Standard CFA DNG with a conventional 2x2 RGB Bayer pattern | In scope for metadata and corpus evidence. |
| CFA DNG with X-Trans, Quad Bayer, four-color, nonstandard `CFALayout`, or another deferred CFA geometry | Deferred and must be rejected explicitly. |
| LinearRaw DNG | Useful as format evidence, but not the initial Bayer decode target. |
| Monochrome or other non-mosaic DNG | Deferred. |

Rawler's broader camera and photometric support is not evidence that `fornax`
supports every DNG payload. Each accepted payload class needs its own validation.

## 2. Findings

### 2.1 Decode representation and sample domain

`[verified]` dnglab's architecture keeps the DNG parser and post-processing code
in the rawler crate. The raw image IFD supplies `ImageWidth`, `ImageLength`,
`SamplesPerPixel`, `BitsPerSample`, and `PhotometricInterpretation`; the decoder
accepts CFA `32803`, LinearRaw `34892`, and BlackIsZero `1`
(`rawler/src/decoders/dng.rs:53-86`).

`[verified]` Decoding does not normalize the buffer. Rawler stores integer or
floating-point samples in `RawImageData`, retains `blacklevel`, `whitelevel`,
`linearization_table`, and DNG opcode lists, and leaves the stored bit depth
unchanged until `apply_scaling()` is called
(`rawler/src/rawimage.rs:203-266`, `:549-576`;
`rawler/src/decoders/dng.rs:87-134`).

`[verified]` `RawImage::apply_scaling()` performs, in order:

1. Apply `LinearizationTable`, if present.
2. Convert samples to `f32`.
3. Subtract black level, normalize by `WhiteLevel - black`, and clip
   underflow to zero.
4. Replace the metadata black level with zero and white level with one.

Sources: `rawler/src/rawimage.rs:546-576`;
`rawler/src/imgop/raw.rs:108-201`. This ordering agrees with the DNG processing
model.

`[verified]` The CFA path currently passes only a four-element black/white array
to `correct_blacklevel_cfa`, so this implementation is explicitly a 2x2 Bayer
path. The general `BlackLevel` type can store a larger repeating pattern, but
`apply_scaling()` does not consume the pattern dimensions or an ActiveArea
origin. Sources: `rawler/src/rawimage.rs:56-132`, `:549-576`;
`rawler/src/imgop/raw.rs:167-201`.

### 2.2 DNG tag mapping

The DNG specification declares linear-reference values as zero-light `0.0` and
maximum useful signal `1.0`. Its processing order is Linearization, Black
Subtraction, Rescaling/Normalization, then Clipping
(DNG 1.7.0.0, "Mapping Raw Values to Linear Reference Values", p. 96).

| DNG tag | Type / count | Default and units | Coordinate/sample meaning | rawler handling |
| --- | --- | --- | --- | --- |
| `Orientation` | TIFF `SHORT`, count 1 | Required for DNG; values follow TIFF 1-8 | Stored-image display orientation; rotation/mirror metadata, not sensor sample transformation | Parsed into `Orientation`; preserved on `RawImage`. The raw buffer remains in stored orientation. `rawler/src/decoders/dng.rs:66-73`; `rawler/src/decoders/mod.rs:390-474` |
| `BitsPerSample` | TIFF `SHORT`, per sample | Integer bits for integer samples; 16, 24, or 32 for floating point | Stored sample bit depth, not a normalized range | Read and retained as `Camera::real_bps` and image data type. `rawler/src/decoders/dng.rs:65-72,344-369` |
| `SampleFormat` | TIFF `SHORT`, per sample | DNG <1.4 implied unsigned integer; 1 unsigned, 3 IEEE FP | Distinguishes integer sensor codes from floating-point samples | Used to select the floating-point WhiteLevel default. `rawler/src/decoders/dng.rs:100-106` |
| `CFARepeatPatternDim` | TIFF/EP `SHORT[2]`: rows, columns | Required with CFA photometric interpretation | Dimensions of the repeating CFA tile whose origin is the top-left of ActiveArea | Tag code exists, but the DNG decoder does not read it; `CFA::new_from_tag` infers dimensions from element count. `rawler/src/tags.rs:131-132`; `rawler/src/cfa.rs:201-225` |
| `CFAPattern` | TIFF/EP `BYTE[rows*columns]` | Required for CFA; values are indices into `CFAPlaneColor` | Row-major CFA tile; origin is ActiveArea top-left | Pattern is converted to `R/G/B/C/M/Y/U`, then shifted by ActiveArea `(left,top)` modulo pattern dimensions so the `RawImage` can use stored-buffer `(0,0)` as its origin. `rawler/src/decoders/dng.rs:451-463` |
| `CFAPlaneColor` | DNG `BYTE[ColorPlanes]` | Optional for common 3-color Bayer behavior | Maps CFA indices to color-plane numbers | Tag is declared but not read by the DNG decoder; rawler therefore assumes the common `0=R,1=G,2=B` mapping. `rawler/src/tags.rs:534`; `rawler/src/cfa.rs:201-214` |
| `CFALayout` | DNG `SHORT` | Default 1 (rectangular/standard) | Describes nonstandard CFA geometry | Tag is declared but not interpreted by this path. `rawler/src/tags.rs:535` |
| `ActiveArea` | `SHORT` or `LONG`, count 4: top, left, bottom, right | Full `ImageLength`/`ImageWidth` | Half-open rectangle of active, non-masked samples in stored-image coordinates | Converted from TLBR to rawler `Rect{x,y,w,h}`, then to left/top/right/bottom border offsets. `rawler/src/decoders/dng.rs:466-482`; `rawler/src/imgop/mod.rs:146-166` |
| `MaskedAreas` | `SHORT` or `LONG`, four values per rectangle: top, left, bottom, right | None | Fully masked rectangles in stored-image coordinates; optional measurement regions for black level | Parser exists in `_get_masked_areas`, but the DNG decode path does not populate `RawImage::blackareas`. `rawler/src/decoders/dng.rs:498-512` |
| `DefaultCropOrigin` | `SHORT`, `LONG`, or `RATIONAL`, count 2: horizontal, vertical | `0,0` | Raw-image coordinates relative to ActiveArea top-left, before DefaultScale | Added to ActiveArea origin when creating `crop_area`. `rawler/src/decoders/dng.rs:331-343,484-495` |
| `DefaultCropSize` | `SHORT`, `LONG`, or `RATIONAL`, count 2: width, height | ActiveArea width and height | Output crop size in raw-image coordinates before DefaultScale; may be fractional | Rawler truncates through `force_usize`; fractional crop origins/sizes are not represented in `Rect`. `rawler/src/decoders/dng.rs:484-495` |
| `BlackLevelRepeatDim` | `SHORT[2]`: rows, columns | `1,1` | Repeating black tile dimensions | Parsed and stored in `BlackLevel {width,height}`. `rawler/src/decoders/dng.rs:395-436` |
| `BlackLevel` | `SHORT`, `LONG`, or `RATIONAL`; count `rows*cols*SamplesPerPixel` | Zero | Zero-light encoding in stored sample units; values are row-column-sample scan order and tile origin is ActiveArea top-left | Parsed into rational levels but later reduced to four Bayer values for scaling. `rawler/src/decoders/dng.rs:395-436`; `rawler/src/rawimage.rs:549-576` |
| `BlackLevelDeltaH` | `SRATIONAL[ActiveArea width]` | All zero | Column black-level delta added to the repeating pattern | Declared in rawler tags but not parsed by the DNG decoder. `rawler/src/tags.rs:539` |
| `BlackLevelDeltaV` | `SRATIONAL[ActiveArea length]` | All zero | Row black-level delta added to the repeating pattern | Declared in rawler tags but not parsed by the DNG decoder. `rawler/src/tags.rs:540` |
| `WhiteLevel` | `SHORT` or `LONG[ SamplesPerPixel ]` | `2^BitsPerSample - 1` for unsigned integer; `1.0` for floating point | Fully saturated stored encoding level, per sample plane | Parsed per channel; single value is replicated when `SamplesPerPixel > 1`. `rawler/src/decoders/dng.rs:438-446` |
| `LinearizationTable` | `SHORT[]` | None | Maps stored integer codes to linear values before black subtraction | Preserved at decode; applied before black/white scaling with a lookup table. `rawler/src/decoders/dng.rs:420-436`; `rawler/src/imgop/raw.rs:203-228` |

Specification support for the defaults and coordinate statements is in DNG
1.7.0.0: CFA photometric interpretation and origin (pp. 21-22), black, white,
and default-crop tag definitions (pp. 26-30), ActiveArea and MaskedAreas (p. 43),
and linear-reference processing (p. 96). The `DefaultCropSize` fallback to the
ActiveArea dimensions is also explicit in the Adobe SDK
(`dng_sdk/source/dng_ifd.cpp:3101-3118`). The half-open rectangle behavior is
confirmed by Adobe's `dng_rect::Contains`, which uses `b > y` and `r > x`
(`agent-work/references/dng_sdk/dng_sdk/source/dng_rect.h:112-125`).

### 2.3 Coordinate spaces and CFA phase

`[verified]` There are three distinct spaces:

- **Stored-image space:** origin at stored raw sample `(0,0)`; width and height
  are the TIFF image dimensions.
- **ActiveArea space:** origin at ActiveArea `(left,top)`, excluding masked
  rows/columns. DNG defines CFA and black-level repeat patterns relative to this
  origin.
- **DefaultCrop space:** `DefaultCropOrigin` is relative to ActiveArea, and
  `DefaultCropSize` is measured in raw-image coordinates before `DefaultScale`.

`[verified]` Rawler rebases the CFA pattern into stored-image coordinates by
shifting left by `left % CFA_width` and down by `top % CFA_height`
(`rawler/src/decoders/dng.rs:451-463`). It also adds the ActiveArea origin to the
default-crop origin (`rawler/src/decoders/dng.rs:331-343`).

`[inference]` Rawler does not apply the same ActiveArea-origin shift to the
black-level repeat pattern before `apply_scaling()`. Consequently, a non-zero
ActiveArea origin or a black pattern larger than the four-value Bayer case can
select the wrong black value. This follows from DNG's specified ActiveArea tile
origin and rawler's `as_bayer_array` reduction. The existing `BlackLevel::shift`
test covers a general pattern shift, but the DNG decode-to-scaling path does not
call it (`rawler/src/rawimage.rs:124-156,549-576,824-836`).

`[verified]` Rawler's CFA dimensions are inferred from the CFA byte count:
4 -> 2x2, 16 -> 8x2, 36 -> 6x6, and 144 -> 12x12; other sizes panic, and a
1x1 monochrome CFA is not represented by this path
(`rawler/src/cfa.rs:216-232`). This is narrower than the general
`CFARepeatPatternDim` format.

`[verified]` Orientation is retained but not applied during raw decoding.
Rotations/mirrors that change buffer geometry would also change CFA phase and
crop coordinates, so a universal representation should preserve orientation
separately rather than silently rotating the sensor buffer.

### 2.4 Processing behavior and DNG opcode boundaries

`[verified]` Rawler applies DNG opcode lists at their specified stages:
OpcodeList1 to stored raw data, List2 immediately after mapping to linear
reference values, and List3 after demosaicing
(`rawler/src/imgop/develop.rs:181-250,440-491`). This ordering is documented in
the DNG specification as a format-level processing model, but opcodes such as
gain maps and bad-pixel correction are optional broader DNG behavior.

`[verified]` The scalar/channel scaling implementation clips only negative
values. Values above the normalized maximum are retained by rawler, whereas the
DNG specification says values above `1.0` should be clipped and permits, but
does not require, clipping below `0.0`
(`rawler/src/imgop/raw.rs:108-201`; DNG 1.7.0.0 p. 96).
`[recommendation]` `fornax` must choose and document its clipping policy in
`SYN-02`; it should not inherit rawler's policy by accident.

`[verified]` `apply_scaling()` mutates the buffer and resets metadata to
black=0/white=1. There is no explicit processing-state marker that would reject
a second invocation. This is a useful warning for `fornax`: metadata alone is
insufficient to prevent double correction once the original levels are reset
(`rawler/src/rawimage.rs:546-576`).

### 2.5 DNG versus vendor RAW metadata

`[verified]` DNG is self-describing for the listed fields: field types, defaults,
and coordinate bases are specified, and the raw IFD must identify CFA or
LinearRaw data (DNG 1.7.0.0 pp. 21-22, 26-30, and 43).

`[verified]` Vendor RAW support in rawler depends on format-specific parsers and
a camera catalog. `Camera` carries catalog overrides such as black/white levels,
active area, crop area, CFA, and hints (`rawler/src/decoders/camera.rs:20-105`);
individual decoders extract makernotes and format-specific offsets. These fields
are not available through one standard vendor metadata vocabulary.

`[inference]` The universal decoder boundary must therefore normalize DNG and
vendor metadata into the same semantic fields while retaining provenance. DNG
defaults can be applied deterministically; vendor fallbacks may depend on camera
identification, catalog revisions, or decoder-specific inference.

### 2.6 Licensing

`[verified]` dnglab/rawler is licensed LGPL-2.1; the repository root `LICENSE`
and `rawler/Cargo.toml` both identify that license. Reusing code in `fornax`
would require satisfying LGPL-2.1 relinking/source-distribution obligations.
This report's recommended reuse is limited to ideas and independently
implemented semantics unless a later licensing decision explicitly permits
code reuse.

`[verified]` The Adobe DNG SDK mirror carries Adobe's SDK license notices in its
source headers, while the DNG specification itself is a copyrighted Adobe
publication with restrictions on reproduction. The specification and SDK are
evidence sources here; no code or document text is proposed for incorporation
into `fornax`.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| DNG raw IFD parsing and accepted photometric values | `rawler/src/decoders/dng.rs:53-134` |
| Stored integer/float samples are not normalized at decode | `rawler/src/rawimage.rs:203-266,549-576`; `rawler/src/decoders/dng.rs:87-134` |
| Linearization/black/rescale/clip order | DNG 1.7.0.0 p. 96; `rawler/src/rawimage.rs:549-576` |
| Black/White tag definitions and defaults | DNG 1.7.0.0 pp. 26-30, `BlackLevel`/`WhiteLevel` |
| Black pattern origin and order | DNG 1.7.0.0 pp. 26-30, `BlackLevelRepeatDim`/`BlackLevel` |
| Rawler BlackLevel parsing and reduction | `rawler/src/decoders/dng.rs:395-446`; `rawler/src/rawimage.rs:56-156` |
| ActiveArea order and default | DNG 1.7.0.0 p. 43 |
| Default crop basis and defaults | DNG 1.7.0.0 pp. 26-30; `agent-work/references/dng_sdk/dng_sdk/source/dng_ifd.cpp:3101-3118` |
| Rawler crop conversion | `rawler/src/decoders/dng.rs:331-343,466-495` |
| CFA origin relative to ActiveArea | DNG 1.7.0.0 pp. 21-22 |
| Rawler CFA rebasing and size inference | `rawler/src/decoders/dng.rs:451-463`; `rawler/src/cfa.rs:170-232` |
| Orientation is preserved, not applied to samples | `rawler/src/decoders/dng.rs:66-73`; `rawler/src/decoders/mod.rs:390-474` |
| Opcode staging | `rawler/src/imgop/develop.rs:181-250,440-491` |
| Underflow/overflow difference | `rawler/src/imgop/raw.rs:108-201`; DNG 1.7.0.0 p. 96 |
| License | `agent-work/references/dnglab/LICENSE`; `agent-work/references/dnglab/rawler/Cargo.toml:8` |
| DNG specification revision | `agent-work/references/dng_sdk/DNG_Spec_1_7_0_0.pdf`, title/version page |

All dnglab paths above are relative to
`agent-work/references/dnglab/`. DNG page references use the pinned PDF at
`agent-work/references/dng_sdk/DNG_Spec_1_7_0_0.pdf`.

## 4. Implications for fornax

`[recommendation]` Preserve stored sample codes and their encoding separately
from any black-corrected representation. The decoder should report integer bit
depth or floating-point sample format, black/white metadata, and processing
state; it should not silently normalize merely because a DNG contains the
tags.

`[recommendation]` Represent black levels with an explicit repeat rectangle and
an origin defined in ActiveArea coordinates. A scalar, per-channel value, and
spatially varying tile must remain distinguishable. Resolve black contribution
before cropping or carry the ActiveArea origin through to preprocessing.

`[recommendation]` Keep `ActiveArea`, `DefaultCropOrigin`, `DefaultCropSize`, and
orientation as separate typed metadata. Do not collapse them into a single crop.
DefaultCrop can be fractional before `DefaultScale`; a universal representation
must either preserve rational coordinates or explicitly reject unsupported
fractional crop geometry.

`[recommendation]` Preserve `CFAPlaneColor` and `CFALayout`; do not assume every
CFA byte value is `R`, `G`, then `B`. The minimum initial implementation may
accept only conventional 2x2 RGB Bayer layouts, but it should reject or retain
other layouts explicitly rather than panic or reinterpret them.

`[recommendation]` Specify clipping independently. The DNG model recommends
clipping above 1.0 and permits preserving negative normalized values; rawler
does the opposite for overflow. This disagreement belongs in `SYN-02`.

`[recommendation]` Add a processing-state/type distinction or equivalent guard
so normalization cannot be applied twice after metadata has been reset.

`[recommendation]` Treat vendor RAW mappings as decoder adapters with provenance.
DNG defaults and vendor catalog fallbacks must not be merged without recording
which source supplied each value.

## 5. Open questions and limitations

- The DNG 1.7.0.0 specification and rawler code do not resolve a single
  project-wide overflow policy; `fornax` must choose one in `SYN-02`.
- The current rawler implementation narrows general BlackLevel patterns to a
  four-value Bayer array and does not clearly apply the ActiveArea-origin shift
  in `apply_scaling()`. Whether another preprocessing path compensates for this
  is `[unverified]`.
- `BlackLevelDeltaH` and `BlackLevelDeltaV` are declared by rawler but not
  parsed by the DNG decoder. Their optional implementation impact is unresolved.
- Fractional `DefaultCropOrigin`/`DefaultCropSize` are accepted by DNG but rawler
  truncates them to integer geometry. `[inference]` This can lose subpixel crop
  information.
- Rawler does not read `CFAPlaneColor` or `CFALayout` in its DNG decode path.
  Four-color, nonstandard-layout, and alternative plane-order CFA files are
  therefore not established as supported.
- DNG opcode support extends beyond the initial `fornax` decode/preprocess/
  demosaic scope. `SYN-01` and `SYN-02` must decide which opcode effects are
  mandatory metadata interpretation versus unsupported processing.
- The Adobe SDK mirror is not the canonical Adobe download endpoint, although it
  contains the 1.7.0.0 PDF and SDK source. The document itself identifies its
  version and publication date.

## 6. References

- dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`.
  - Local checkout: `agent-work/references/dnglab/`.
  - `rawler/src/decoders/dng.rs`, `rawler/src/rawimage.rs`,
    `rawler/src/imgop/raw.rs`, `rawler/src/imgop/develop.rs`,
    `rawler/src/cfa.rs`, `rawler/src/tags.rs`, `rawler/src/decoders/mod.rs`.
  - License: `agent-work/references/dnglab/LICENSE` (LGPL-2.1).
- Adobe DNG SDK mirror: `https://github.com/emmcb/adobe-dng-sdk.git` at
  `962a7028efdc8f8a651cf6335c3ec53053472339`.
  - Local checkout: `agent-work/references/dng_sdk/`.
  - Specification: `DNG_Spec_1_7_0_0.pdf`, version 1.7.0.0, June 2023.
  - Reference implementation: `dng_sdk/source/dng_ifd.cpp`,
    `dng_sdk/source/dng_linearization_info.cpp`,
    `dng_sdk/source/dng_rect.h`.
- Specification sections used: CFA photometric interpretation and orientation
  (pp. 21-22), DNG tag definitions for black/white/crop (pp. 26-30), ActiveArea
  and MaskedAreas (p. 43), and "Mapping Raw Values to Linear Reference Values"
  (p. 96).
