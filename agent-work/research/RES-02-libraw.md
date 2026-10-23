# LibRaw

> **Task:** `RES-02`
> **Owner:** RES-02 research agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-23
> **Upstream versions:** LibRaw tag `0.22.2`, peeled commit
> `b93f6e45c194f5df9b02a43b1af9a54b4f41f33f`; local `fornax` wrapper at the
> current inspected worktree.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Each relevant field is documented with type, unit, coordinate
>   convention, sentinel values, and validity conditions.
> - [x] Black-level repeat-pattern and per-channel semantics are explained,
>   including alignment with sensor coordinates and residual ambiguities.
> - [x] The difference between `raw_image` and processed output is stated.
> - [x] A field-to-universal-candidate mapping is provided.
> - [x] Target-format coverage is separated from deferred formats and non-Bayer
>   payloads.
> - [x] Unverified claims are labeled `[inference]`.
> - [x] The report contains the six required sections and cites sources.
>
> **Verification snapshot:** Primary checks completed 2026-10-23; GitHub pull-request review is
> pending. The buffer meanings, `cblack` layout, ActiveArea copy/crop rebasing,
> 1x1/2x2 folding, inline versus fallback subtraction, white-level fields,
> metadata gap, and wrapper behavior were checked against LibRaw 0.22.2 source
> and the local wrapper.

## 1. Scope and questions

This report traces LibRaw 0.22.2 from file opening and unpacking through the
raw-data API and raw-to-image preprocessing, then compares those semantics with
the existing safe `libraw` wrapper in `fornax`.

The questions were:

- How are RAW files opened, identified, unpacked, and exposed?
- Does `rawdata.raw_image` contain stored sensor codes or black-corrected data?
- How are dimensions, pitch, sample type, CFA pattern, and CFA origin encoded?
- How are scalar, per-channel, repeating, and spatially varying black levels
  represented and applied?
- What do saturation/white-level fields mean?
- How do margins, active area, crop, orientation, and masks affect coordinates?
- Which metadata survives, and which does the existing wrapper expose?
- What should a universal `fornax` representation preserve or reject?

The source now lives at `agent-work/references/libraw/`; the previous
installed-header-only limitation has been resolved by inspecting the exact
0.22.2 release source.

The initial implementation scope is DNG, Sony ARW, Nikon NEF, and Canon `CR*`
payloads containing a conventional 2x2 RGB Bayer mosaic. This report uses
LibRaw to establish shared RAW semantics for that scope:

| Coverage class | Treatment in this report |
| --- | --- |
| Bayer DNG | In scope for metadata, black/white, CFA, crop, and sample-domain evidence. |
| Sony ARW | In scope for the shared semantics above; vendor decoder behavior is not independently fixture-verified here. |
| Nikon NEF | In scope for the shared semantics above; vendor decoder behavior is not independently fixture-verified here. |
| Canon `CR*` | In scope for the shared semantics above; `CR2`/`CR3` decoder behavior is not independently fixture-verified here. |
| Fujifilm X-Trans / RAF | Deferred future work; evidence is retained only to define rejection boundaries. |
| Sigma Foveon/X3 / X3F | Deferred future work; evidence is retained only to define rejection boundaries. |
| Quad Bayer, four-color, monochrome, non-mosaic, and other RAW formats | Deferred and outside the initial task. |

`[inference]` LibRaw's broad camera catalog is not evidence that every target
format/camera already produces the required sample and metadata semantics in
`fornax`. Decoder integration must verify each target fixture.

## 2. Findings

### 2.1 Source and licensing

`[verified]` LibRaw is offered under LGPL-2.1 or CDDL-1.0 at the user's option
(`LICENSE.LGPL`, `LICENSE.CDDL`, and source headers). This report recommends
reuse through the existing adapter or independent implementation rather than
copying internal structures into `fornax-core`.

`[verified]` The checked-out tag resolves to
`b93f6e45c194f5df9b02a43b1af9a54b4f41f33f`; the repository was clean when
inspected.

### 2.2 Decode lifecycle and buffer meaning

`[verified]` LibRaw's normal path is open/identify, `unpack`, then conversion to
the four-channel working image (`raw2image`/`dcraw_process`). The C API exposes
these as `libraw_open_file`, `libraw_unpack`, `libraw_raw2image`,
`libraw_subtract_black`, and `libraw_dcraw_process`
(`libraw/libraw.h:91-180`; `src/libraw_c_api.cpp:53-66,149-156,294-300`).

`[verified]` `libraw_rawdata_t.raw_alloc` owns the allocated raw bitmap.
`raw_image`, `color4_image`, `color3_image`, `float_image`, and the float
multichannel pointers are aliases into that allocation
(`libraw/libraw_types.h:993-1018`). The safe wrapper therefore must retain the
LibRaw owner for every borrowed raw buffer.

`[verified]` `rawdata.raw_image` is a single-channel `ushort*`, while
`rawdata.color4_image` is a four-channel `ushort (*)[4]`. The variants
distinguish unpacked sensor codes/native samples from:

- the later `imgdata.image`, a four-channel `ushort (*)[4]` working buffer;
- fully processed RGB output produced by `dcraw_process`.

Sources: `libraw/libraw_types.h:993-1065`; `src/preprocessing/raw2image.cpp:219-330`;
`src/postprocessing/dcraw_process.cpp:43-181`.

`[verified]` `unpack()` retains the raw bitmap and raw metadata. Black
subtraction is applied later when `raw2image`/`subtract_black` copies into
`imgdata.image`, or inline by `raw2image_ex(true)` for supported Bayer paths
(`src/preprocessing/raw2image.cpp:415-560`;
`src/postprocessing/dcraw_process.cpp:42-91`).

`[inference]` `raw_image` should be treated as unpacked stored sample codes, not
as a promise of a particular physical unit or bit depth across every decoder.
Some vendor decoders normalize, scale, unpack packed values, or incorporate
format-specific corrections before assignment. The universal representation
needs explicit sample encoding and provenance rather than assuming all
LibRaw raw buffers are equivalent.

### 2.3 Dimensions, stride, and crop coordinates

`[verified]` `libraw_image_sizes_t` exposes:

- `raw_height`, `raw_width`: full allocated sensor bitmap dimensions;
- `height`, `width`: selected output/active dimensions;
- `top_margin`, `left_margin`: offsets into the allocated bitmap;
- `raw_pitch`: raw-buffer row stride in bytes;
- `flip`: orientation value.

Sources: `libraw/libraw_types.h:215-225`; prior installed-header audit retained
the same declarations. The safe wrapper exposes these fields individually
(`crates/libraw/src/data_structure/image_sizes.rs`).

`[verified]` `raw2image` copies rows using
`raw_pitch / 2` and offsets each row by `left_margin/top_margin`; output uses
the selected `S.width` and `S.height`
(`src/preprocessing/raw2image.cpp:219-330,415-560`).

`[verified]` The safe wrapper's `Rawdata::get_rawdata` constructs buffers from
`raw_width` and `raw_height` but does not use `raw_pitch`
(`crates/libraw/src/data_structure/rawdata.rs`). `[inference]` This is only safe
if the selected LibRaw decoder guarantees a contiguous raw allocation of
exactly `raw_width * raw_height` elements. The wrapper should validate or carry
the pitch for a universal representation.

### 2.4 CFA representation and origin

`[verified]` `libraw_iparams_t.filters` is a packed 32-bit CFA descriptor.
`LibRaw::FC(row,col)` extracts a 2-bit color index from it; `filters == 0`
indicates no CFA and `filters == 9` selects the X-Trans table
(`libraw/libraw.h:300-315`; `src/utils/utils_dcraw.cpp:21-45`).

`[verified]` `fcol(row,col)` includes `top_margin` and `left_margin` for the
special 16x16 filter case and otherwise delegates to `FC`/X-Trans lookup
(`src/utils/utils_dcraw.cpp:21-45`). This is the main mechanism that preserves
sensor phase when active-area margins are removed.

`[verified]` `cdesc` describes the color labels, while `colors`, `filters`, and
the X-Trans table describe how mosaic positions map to color planes
(`libraw/libraw_types.h:188-208`). The safe wrapper exposes `filters`,
`dng_version`, `colors`, X-Trans dimensions/table, and `cdesc`
(`crates/libraw/src/data_structure/iparams.rs`).

`[inference]` A universal model should not expose LibRaw's packed `filters`
integer. It should convert it to a typed CFA descriptor with explicit origin;
otherwise crop rebasing is easy to lose.

### 2.5 Black-level representation

`[verified]` `libraw_colordata_t` contains:

- `unsigned black`: common scalar black offset;
- `unsigned cblack[4104]`: black-level metadata;
- `unsigned data_maximum`: maximum observed sample after subtraction;
- `unsigned maximum`: saturation/white point;
- `unsigned linear_max[4]`: per-channel linear maximum.

Sources: `libraw/libraw_types.h:781-803`; `libraw/libraw_const.h:85`.

`[verified]` `cblack` layout is:

| Index | Meaning |
| --- | --- |
| `cblack[0..3]` | Per-CFA-color offsets. During `adjust_bl`, the common minimum is moved into `black` and then added back to all four entries, so subtraction applies these values plus `black` through the per-channel term. These indices follow LibRaw's CFA color mapping, including a separate green slot where applicable. |
| `cblack[4]` | Repeating black-pattern row count (`BlackLevelRepeatRows`). |
| `cblack[5]` | Repeating black-pattern column count (`BlackLevelRepeatCols`). |
| `cblack[6..6+rows*cols-1]` | Row-major spatial tile of black offsets. |
| `cblack[4103]` | In DNG parsing, the last slot may temporarily store the parsed black-value count. |

Sources: DNG parse in `src/metadata/tiff.cpp:1115-1295`;
`src/preprocessing/subtract_black.cpp:20-72`;
`src/utils/utils_libraw.cpp:468-550`.

`[verified]` `adjust_bl()` canonicalizes overlapping representations:

- user scalar/per-channel black overrides clear the repeat pattern;
- 1x1 and 2x2 repeat patterns are folded into per-color offsets for a normal
  Bayer `filters > 1000` layout;
- a common minimum is removed from per-channel and repeat values and moved into
  `black`;
- `black` is then added back to `cblack[0..3]`, leaving `cblack` as the
  per-pixel correction source used during subtraction.

Source: `src/utils/utils_libraw.cpp:468-550`.

`[verified]` For a 2x2 pattern, the folding code maps each row-major tile entry
to its CFA color using `FC(c / 2, c % 2)` before clearing
`cblack[4]`/`cblack[5]` (`src/utils/utils_libraw.cpp:487-507`). The initial
conventional Bayer path therefore reduces the spatial tile to the four per-CFA
values before normal black subtraction. A 1x1 tile follows the same fold and
applies its value to each color.

`[verified]` `subtract_black_internal()` computes:

```text
repeat_black = cblack[6 + (row % repeat_rows) * repeat_cols + col % repeat_cols]
corrected = CLIP(image[row,col,channel] - repeat_black - cblack[channel])
```

over the four-channel `imgdata.image` working buffer. It updates
`data_maximum`, reduces `maximum` by `black`, then zeros `cblack` and `black`.
Sources: `src/preprocessing/subtract_black.cpp:20-84`.

`[verified]` The repeat-pattern index in `subtract_black_internal` uses the
processed-image coordinates `q / S.iwidth` and `q % S.iwidth`; there is no
explicit ActiveArea/margin term in that formula
(`src/preprocessing/subtract_black.cpp:38-54`).

`[verified]` LibRaw does not retain a separate repeat-pattern origin field.
However, DNG defines the black tile origin at the top-left of `ActiveArea`
(`RES-03` §2.2–2.3, mapping the DNG 1.7.0.0
`BlackLevelRepeatDim`/`BlackLevel` rules). In the default conversion path,
`raw2image_ex` copies the active rectangle starting at
`top_margin`/`left_margin` into output coordinate `(0,0)`, so `q=0` corresponds
to the ActiveArea top-left when no user crop is requested
(`src/preprocessing/raw2image.cpp:273-298,423-435`).

`[verified]` A user crop changes the relationship. `raw2image_ex` adds the crop
offset to `S.top_margin`/`S.left_margin`, rewrites the CFA filter phase, and
copies the cropped rectangle into a new image beginning at `(0,0)`
(`src/preprocessing/raw2image.cpp:338-393`). A still-spatial general tile would
therefore be indexed relative to the crop origin, not the original ActiveArea
origin.

`[verified]` The normal `dcraw_process` inline path calls `raw2image_ex(true)`.
That path subtracts only `cblack[0..3]`, then clears those four entries; it does
not subtract `cblack[6..]` while copying. The later
`subtract_black_internal` pass, which does consume `cblack[6..]`, is called only
when inline subtraction was disabled or `data_maximum == 0`
(`src/preprocessing/raw2image.cpp:423-431,554-561`;
`src/postprocessing/dcraw_process.cpp:43-87`).

`[inference]` For normal Bayer data, the 1x1/2x2 fold makes the supported target
well-defined: black is represented as common scalar plus per-CFA offsets, and
the ActiveArea-relative tile phase is absorbed into those offsets. For a larger
or otherwise spatial tile, the inspected normal path neither preserves an
explicit tile origin nor establishes repeat-tile subtraction after a user crop.
Such patterns must be rejected or remain in LibRaw-specific metadata until a
separate task verifies and implements ActiveArea-origin-aware behavior.

`[verified]` Subtracting black mutates the working image and resets the black
metadata. `subtract_black_internal` explicitly zeros `cblack` and `black`;
`raw2image` also zeros the four per-channel entries after inline subtraction
(`src/preprocessing/subtract_black.cpp:64-72`;
`src/preprocessing/raw2image.cpp:550-561`). Lack of original metadata cannot
prove that subtraction has not already happened.

`[verified]` The DNG-specific source retains `dng_levels.dng_black`,
`dng_cblack[4104]`, floating-point counterparts, `dng_whitelevel[4]`,
`default_crop[4]`, and `user_crop[4]`
(`libraw/libraw_types.h:254-270`). The wrapper currently does not expose these
typed DNG fields.

### 2.6 White level and saturation

`[verified]` `color.maximum` is LibRaw's saturation/white point used by
processing; `color.linear_max[4]` is a per-channel linear maximum used,
especially for DNG/vendor metadata. `data_maximum` is derived from observed
values after black subtraction.

`[verified]` `adjust_maximum()` may lower `maximum` toward `data_maximum` when
the observed maximum is close enough to the configured threshold
(`src/utils/utils_libraw.cpp:448-469`). This means `maximum` can be an inferred
or adjusted value, not always the raw file's declared saturation.

`[inference]` A universal representation should distinguish:

- declared white/saturation level;
- decoder-adjusted processing maximum;
- measured data maximum;
- normalized `1.0` after preprocessing.

Collapsing these into one field would lose provenance and explainability.

### 2.7 Metadata retention and the safe wrapper

`[verified]` LibRaw retains camera make/model, normalized names, CFA and color
fields, dimensions/margins, WB matrices, black/white levels, DNG levels, and
format-specific metadata in its large C structures. It does not expose one
stable vendor-neutral metadata map.

`[verified]` The existing safe wrapper exposes a useful subset through
`ImageSizes` and `IParams`, including make/model, `raw_count`, Foveon/DNG flags,
CFA filters, X-Trans tables, color count, and description
(`crates/libraw/src/data_structure/image_sizes.rs`,
`crates/libraw/src/data_structure/iparams.rs`).

`[verified]` The wrapper does not expose `color.black`, `color.cblack`,
`color.maximum`, `color.linear_max`, or `dng_levels` accessors. It does expose a
direct C call for `libraw_subtract_black` and raw-to-image/process operations
(`crates/libraw/src/c_api/auxiliary.rs`).

`[inference]` Exposing `libraw_subtract_black` without exposing the original
metadata, sample-domain state, or margin-aware repeat origin makes double
correction easy. The adapter should snapshot metadata before mutation and
represent corrected samples with a distinct type/state.

### 2.8 Field-to-universal-candidate mapping

| LibRaw field/API | Candidate universal meaning | Required handling |
| --- | --- | --- |
| `rawdata.raw_image` / `color4_image` | Sensor-domain sample buffer | Preserve sample type, dimensions, pitch, and provenance; do not call it normalized. |
| `sizes.raw_width/height` | Allocated raw dimensions | Keep separate from active/output dimensions. |
| `sizes.width/height` | Active/selected dimensions | Define exact relation to margins and crop. |
| `sizes.left_margin/top_margin` | Active-area origin in raw allocation | Preserve origin for CFA and black-pattern alignment. |
| `sizes.raw_pitch` | Row stride in bytes | Validate against width/sample type; never discard silently. |
| `sizes.flip` | Orientation | Preserve as metadata; do not rotate sensor buffer implicitly. |
| `idata.filters` + X-Trans table | Typed CFA descriptor | Convert to explicit layout, channel mapping, and origin; reject unsupported layouts. |
| `color.black` | Common black offset | Part of black model, not necessarily the complete level. |
| `color.cblack[0..3]` | Per-channel offsets | Preserve as typed values after understanding LibRaw's canonicalization. |
| `color.cblack[4..5]` / `[6..]` | Repeat-pattern shape and tile | Preserve with explicit coordinate origin; do not reduce to one scalar. |
| `color.maximum`, `linear_max`, `data_maximum` | Saturation/adjusted/observed maxima | Preserve provenance and never conflate them. |
| `dng_levels.default_crop/user_crop` | DNG crop metadata | Map to typed crop only after `RES-03`/`SYN-01` reconciliation. |
| WB/color-matrix fields | Capture metadata | Preserve but do not apply in decode/preprocess/demosaic scope. |
| `imgdata.image` after black subtraction | Mutable working image | Must carry corrected-state semantics to prevent double subtraction. |

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| LibRaw release and dual license | Git tag `0.22.2`; `LICENSE.LGPL`; `LICENSE.CDDL` |
| Raw allocation aliases | `agent-work/references/libraw/libraw/libraw_types.h:993-1018` |
| Dimensions, pitch, margins, flip | `agent-work/references/libraw/libraw/libraw_types.h:215-225`; wrapper image-size accessors |
| CFA packed encoding and X-Trans sentinel | `agent-work/references/libraw/libraw/libraw.h:300-315`; `src/utils/utils_dcraw.cpp:21-45` |
| Black/white/linear maximum fields | `agent-work/references/libraw/libraw/libraw_types.h:781-803` |
| DNG level fields | `agent-work/references/libraw/libraw/libraw_types.h:254-270` |
| DNG black parser | `agent-work/references/libraw/src/metadata/tiff.cpp:1115-1295` |
| DNG black-tile origin at ActiveArea top-left | `RES-03` §2.2–2.3; DNG 1.7.0.0 PDF, `BlackLevelRepeatDim`/`BlackLevel` tag definitions |
| Black normalization/canonicalization | `agent-work/references/libraw/src/utils/utils_libraw.cpp:468-550` |
| Conventional 1x1/2x2 CFA fold | `agent-work/references/libraw/src/utils/utils_libraw.cpp:487-507` |
| Black subtraction formula and metadata reset | `agent-work/references/libraw/src/preprocessing/subtract_black.cpp:20-84` |
| ActiveArea-to-image origin and crop rebasing | `agent-work/references/libraw/src/preprocessing/raw2image.cpp:273-298,338-393,423-435` |
| Inline per-channel subtraction leaves spatial tile | `agent-work/references/libraw/src/preprocessing/raw2image.cpp:423-431,554-561` |
| General-tile fallback condition | `agent-work/references/libraw/src/postprocessing/dcraw_process.cpp:43-87` |
| Raw-to-image copy and margins | `agent-work/references/libraw/src/preprocessing/raw2image.cpp:219-330,415-561` |
| Maximum adjustment | `agent-work/references/libraw/src/utils/utils_libraw.cpp:448-469` |
| DNG row/column black delta averaging | `agent-work/references/libraw/src/metadata/tiff.cpp:1338-1348` |
| Wrapper raw conversion does not use pitch | `crates/libraw/src/data_structure/rawdata.rs` |
| Wrapper mutation surface | `crates/libraw/src/c_api/auxiliary.rs` |

## 4. Implications for fornax

`[recommendation]` Define a universal sensor buffer with explicit sample
encoding, width/height, stride, ownership, and processing state. Treat
`raw_image` as decoder evidence, not as the universal type.

`[recommendation]` Preserve raw dimensions and active-area origin separately.
CFA phase and black-pattern phase must be derived from sensor/active coordinates,
not reset at the cropped output origin.

`[recommendation]` Model black levels as a typed union or structure that can
represent scalar, per-CFA, and repeat-tile values. Define the tile origin
explicitly in ActiveArea coordinates. The initial normal-Bayer implementation
may canonicalize 1x1/2x2 tiles to per-CFA values, but it must not pretend that
arbitrary spatial tiles have been supported.

`[recommendation]` Make black subtraction consume a stored-sensor representation
and return a distinct corrected representation. The adapter must remember the
operation in state; resetting LibRaw metadata to zero is not sufficient
evidence for `fornax`.

`[recommendation]` Preserve declared saturation, decoder-adjusted maximum, and
measured maximum separately if the source exposes them. If that is too much for
the first public API, retain the others in extensible metadata rather than
discarding provenance.

`[recommendation]` Snapshot LibRaw black/white and geometric metadata before
calling `raw2image`, `subtract_black`, or `dcraw_process`. The current wrapper
allows those mutating calls, and LibRaw clears black metadata after correction.

`[recommendation]` Reject black patterns that require a spatial origin beyond
the folded conventional Bayer case unless the adapter can apply them before any
crop and preserve the ActiveArea origin explicitly.

`[recommendation]` Fix or validate `raw_pitch` handling in the LibRaw adapter.
At minimum, reject noncontiguous layouts that the current wrapper cannot
represent.

`[recommendation]` Keep LibRaw-specific structs and packed CFA integers behind
the adapter. Convert to typed common metadata while retaining unknown
format-specific fields where practical.

## 5. Open questions and limitations

- LibRaw has no explicit repeat-pattern origin field. Conventional 1x1/2x2
  DNG Bayer tiles are folded into per-CFA values and are within the verified
  initial scope. General spatial tiles are not fully supported by the inspected
  normal conversion path, especially after a user crop.
- `cblack[6..]` is a spatial black tile but does not include a per-plane
  dimension in the subtraction index; per-plane offsets come from `cblack[0..3]`.
  This matches canonical LibRaw Bayer usage but is not a general DNG
  `row*col*sample` structure for arbitrary multichannel data.
- `maximum` may be adjusted from observed data depending on
  `adjust_maximum_thr`; downstream consumers must not assume it always equals a
  file-declared white level.
- The current wrapper ignores `raw_pitch`; whether every supported decoder
  produces a contiguous buffer of exactly `raw_width * raw_height` samples is
  `[unverified]`.
- The wrapper lacks typed accessors for black/white/DNG crop metadata. This is a
  concrete implementation gap, not evidence that LibRaw lacks the values.
- Some LibRaw decoders may perform format-specific preprocessing before the raw
  buffer is exposed. The universal decoder interface must describe such sample
  transformations instead of assuming a uniform "stored code" meaning for all
  formats.
- DNG `BlackLevelDeltaH`/`BlackLevelDeltaV` are not preserved as row/column
  vectors by this path; LibRaw accumulates their average into the scalar black
  level (`src/metadata/tiff.cpp:1338-1348`). Spatially varying deltas remain
  deferred with general black-pattern support.

## 6. References

- LibRaw: `https://github.com/LibRaw/LibRaw.git`, tag `0.22.2`, peeled commit
  `b93f6e45c194f5df9b02a43b1af9a54b4f41f33f`.
  - Local checkout: `agent-work/references/libraw/`.
  - `libraw/libraw.h`, `libraw/libraw_types.h`, `libraw/libraw_const.h`.
  - `src/metadata/tiff.cpp`, `src/preprocessing/subtract_black.cpp`,
    `src/preprocessing/raw2image.cpp`, `src/postprocessing/dcraw_process.cpp`,
    `src/utils/utils_libraw.cpp`, `src/utils/utils_dcraw.cpp`.
  - License: LGPL-2.1 or CDDL-1.0.
- Local wrapper:
  - `crates/libraw/src/data_structure/rawdata.rs`
  - `crates/libraw/src/data_structure/image_sizes.rs`
  - `crates/libraw/src/data_structure/iparams.rs`
  - `crates/libraw/src/c_api/auxiliary.rs`
- DNG coordinate evidence:
  - `agent-work/research/RES-03-dnglab.md` (`RES-03`), §2.2–2.3.
  - `agent-work/references/dng_sdk/DNG_Spec_1_7_0_0.pdf`, DNG 1.7.0.0
    `BlackLevelRepeatDim` and `BlackLevel` definitions for tile origin and
    ordering.
