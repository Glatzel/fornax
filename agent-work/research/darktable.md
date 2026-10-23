# darktable and RAW Preprocessing

> **Task:** `RES-04`
> **Owner:** RES-04 research agent
> **Status:** `review` (ready for user review)
> **Last updated:** 2026-10-23
> **Upstream versions:** darktable `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`
> (`master`, 2026-10-22), inspected from
> `agent-work/references/darktable/`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] The pre-demosaic pipeline order is documented with source paths and
>   symbols.
> - [x] Black-level subtraction, clipping, normalization, and masked-pixel
>   handling are described numerically.
> - [x] The boundary between sensor-domain preprocessing and later image
>   processing is explicit.
> - [x] White balance, color matrices, tone mapping, denoising, and display
>   rendering are identified as out of scope for `fornax`.
> - [x] Contradictions and differences relevant to LibRaw/DNG are recorded for
>   `SYN-01` and `SYN-02`.
> - [x] The report contains the six required sections and cites sources.
>
> **Review snapshot:** RES-04 research agent and primary checks completed
> 2026-10-23; user review is pending. Module order, `rawprepare` formula and CFA
> indexing, crop/mask behavior, the demosaic boundary, out-of-scope operations,
> source differences, and licensing were checked against the pinned darktable
> revision.

## 1. Scope and questions

This report examines darktable's actual RAW pipeline around `rawprepare`,
`temperature`, `highlights`, and `demosaic`. It records the sample-domain
meaning of each step, the coordinate model used for cropping and CFA selection,
and the exact point where sensor-domain processing ends.

The questions were:

- Which modules execute before demosaicing, and in what order?
- How does `rawprepare` normalize sensor samples and select black/white levels?
- How are crop, margins, masks, CFA pattern, and CFA phase handled?
- Does darktable clip underflow or overflow in `rawprepare`?
- Where do white balance and other later operations occur?
- What does the name "darkroom" mean in this project?
- Which behavior conflicts with or differs from LibRaw and DNG semantics?
- Which parts are useful as evidence rather than code to reuse?

The inspected project is **darktable**, not a separate project named
"darkroom". Source line 20 of `src/views/darkroom.c` identifies `darkroom` as
the darktable UI view for developing an image; it is not a separate RAW
processor.

The actionable scope is conventional 2x2 RGB Bayer data in the initial DNG,
Sony ARW, Nikon NEF, and Canon `CR*` files. X-Trans, Foveon, Quad Bayer,
four-color, monochrome, and other sensor paths are treated as deferred
comparison evidence, not as functions `fornax` should implement implicitly.

## 2. Findings

### 2.1 Current RAW pipeline order

`[verified]` The current `v50_order` for RAW images places the relevant modules
in this order:

```text
rawprepare (1.0)
invert (2.0)
temperature (3.0)
rasterfile (3.1)
highlights (4.0)
cacorrect (5.0)
hotpixels (6.0)
rawdenoise (7.0)
demosaic (8.0)
demosaicscale (8.5)
```

Source: `src/common/iop_order.c:306-316`. The ordering constraints explicitly
forbid placing `temperature` before `rawprepare`, `highlights` before
`temperature`, or `demosaic` before `rawdenoise`
(`src/common/iop_order.c:829-837`).

`[verified]` Several entries are optional sensor/channel steps:
`invert` is for negative/scanner-style material, while `cacorrect`,
`hotpixels`, and `rawdenoise` are conditionally selected. For a conventional
Bayer RAW, the mandatory chain of interest is:

```text
decode -> rawprepare normalization/crop -> temperature/WB -> highlights
       -> demosaic
```

`[verified]` `temperature` is not part of the required `fornax` boundary. It
multiplies every mosaic sample by the coefficient selected for that sample's
CFA color, preserving the single-channel mosaic while balancing channels
(`src/iop/temperature.c:508-624`). `fornax` must not perform this operation
merely because WB metadata exists.

### 2.2 `rawprepare` inputs and output

`[verified]` `rawprepare` has four crop borders, four per-channel black levels,
one white level, and an optional embedded gain-map mode
(`src/iop/rawprepare.c:46-55`). The defaults are loaded from image metadata:
`crop_x`, `crop_y`, `crop_right`, `crop_bottom`,
`raw_black_level_separate[0..3]`, and `raw_white_point`
(`src/iop/rawprepare.c:786-804`).

`[verified]` The source image loader fills those metadata values from rawspeed.
Rawspeed's black level and white point become `img->raw_black_level` and
`img->raw_white_point`, and its four black-level-separate values become
`img->raw_black_level_separate[0..3]`
(`src/imageio/imageio_rawspeed.cc:246-271`).

`[verified]` The rawprepare CPU path accepts uint16 or unnormalized float mosaic
input and always writes float mosaic output. It also has a separate path for
pre-downsampled multi-channel buffers (`src/iop/rawprepare.c:328-430`). The
normalization constants are prepared as:

```text
sub[i]  = black[i]
div[i]  = white - black[i]
rdiv[i] = 1 / div[i]
```

for each of four CFA/buffer channels (`src/iop/rawprepare.c:724-747`). A float
image that is already normalized disables the module
(`src/iop/rawprepare.c:598-610,765-767`).

### 2.3 Numerical black-level correction and normalization

`[verified]` For a raw mosaic the per-sample operation is:

```text
n    = sample - black[id]
out  = n / (white - black[id])
```

where `id` is the 2x2 CFA position selected from sensor coordinates. The CPU
implementation uses a reciprocal and fused-multiply-add residual to compute the
division while remaining consistent with the OpenCL path
(`src/iop/rawprepare.c:352-366`; `data/kernels/basic.cl:33-53`).

`[verified]` `rawprepare` writes the result as `float` and sets
`processed_maximum[k] = 1.0` for all four channels; it does not clamp negative
results or values above one inside `rawprepare`
(`src/iop/rawprepare.c:352-366,468-472`). Later modules, such as
`highlights`, may transform or clip values, but that is outside the required
`fornax` preprocessing stage.

`[inference]` The four-value representation means darktable's `rawprepare`
supports scalar and 2x2 per-CFA-position black/white levels. The inspected
module parameters do not contain a general `BlackLevelRepeatDim`, row/column
delta vectors, or an arbitrary spatial black map. Darktable can consume the
decoder's four levels but does not itself implement the full DNG repeat-pattern
model.

`[verified]` When no black-level-separate metadata is supplied, rawspeed may
sample masked areas to calculate them. The loader contains a comment that
rawspeed's black-area handling is currently broken, and calls
`calculateBlackAreas()` only when separate black levels do not already exist
(`src/imageio/imageio_rawspeed.cc:248-250`). `rawprepare` itself does not
measure masked pixels; it uses the metadata snapshot it receives.

`[inference]` Masked-area measurement is therefore not a dependable black-level
source in this revision. The initial `fornax` path should require explicit
black-level metadata or a separately verified fallback rather than reproducing
darktable's provisional rawspeed behavior.

`[verified]` `MaskedAreas`/masked borders are generally excluded by cropping,
not by treating masked samples as image pixels. The crop params come from the
decoder's crop offset and dimensions, and `rawprepare` removes the left/top
margin while reading the source and trims right/bottom dimensions
(`src/imageio/imageio_rawspeed.cc:321-339`;
`src/iop/rawprepare.c:257-285`).

### 2.4 CFA indexing and crop-phase preservation

`[verified]` `rawprepare` selects the CFA black/white slot with

```text
id = 2 * ((row + roi_out.y + top) & 1) + ((col + roi_out.x + left) & 1)
```

(`src/iop/rawprepare.c:316-324`). The addition of output ROI position and crop
top/left means the slot follows the original sensor phase rather than resetting
to `0,0` at the cropped image's first pixel.

`[verified]` After processing, the Bayer filter descriptor is shifted by the
crop offsets using `dt_rawspeed_crop_dcraw_filters`, and X-Trans's 6x6 filter
table is rotated by `(crop_x % 6, crop_y % 6)`
(`src/iop/rawprepare.c:368-373,300-312`).

`[verified]` The crop operation is a spatial translation, not a rescaling:
`modify_roi_out` reduces width/height by left+right/top+bottom
(`src/iop/rawprepare.c:257-285`).

`[inference]` It is unsafe to crop first and then choose black values using
local cropped coordinates. Correct CFA-aware black selection needs the stored
sensor origin or an equivalent phase offset. This is the same problem that
`RES-03` identifies in a more general DNG repeat-pattern context.

### 2.5 Optional embedded gain maps and other pre-demosaic modules

`[verified]` `rawprepare` can multiply normalized samples by a four-channel
embedded DNG gain map, with bilinear map sampling in normalized image
coordinates (`src/iop/rawprepare.c:430-468`; `data/kernels/basic.cl:55-97`).
The map is applied after black subtraction/normalization. It is optional and
its enablement is selected from image metadata.

`[verified]` `highlights` executes after `rawprepare` and `temperature` but
before `demosaic` in the default RAW order
(`src/common/iop_order.c:306-316`). It can reconstruct or clip highlight
information in the mosaic domain. `fornax` has not selected a highlight-
reconstruction policy, so this report records it as adjacent evidence rather
than a required preprocessing operation.

`[verified]` `hotpixels`, `rawdenoise`, and chromatic-aberration correction
also appear in darktable's pre-demosaic order but are optional and outside the
currently specified minimum `fornax` pipeline. `fornax` should add them only
after separate semantic research and user-approved design.

### 2.6 Demosaic boundary and later processing

`[verified]` `demosaic` receives a single-channel float mosaic after
`rawprepare`, `temperature`, and any optional pre-demosaic modules. It reads
the current filter descriptor and, for a full-scale run, produces a
four-float-channel working buffer; preview/zoom paths can use approximate
interpolation (`src/iop/demosaic.c:699-755`).

`[verified]` The default Bayer demosaic method in this revision is RCD; the
module also exposes AMaZE, LMMSE, VNG4, PPG, Markesteijn, and specialized
monochrome/4-Bayer paths (`src/iop/demosaic.c:152-172,928-971,1586-1596`).

`[verified]` The following processing is deliberately outside `fornax`:

- `temperature` white-balance multiplication,
- `colorin`/camera-to-working-space matrix application,
- `exposure`, tone mapping, and curves,
- denoising and sharpening,
- display/output profile conversion and gamma.

The module order places these after demosaic or inside the broader RAW
development pipeline (`src/common/iop_order.c:306-400`). `fornax` must preserve
the metadata needed for them but must not apply them.

### 2.7 Relationship to LibRaw and DNG

| Topic | darktable behavior | LibRaw/DNG comparison |
| --- | --- | --- |
| Black-level shape | Four per-CFA slots in `rawprepare`; rawspeed supplies a summarized four-value array | DNG permits a row-by-column repeating pattern plus `BlackLevelDeltaH/V`; LibRaw exposes scalar, per-channel, and larger internal black arrays. Darktable does not retain the full DNG pattern in `rawprepare`. |
| Normalization | `(sample - black) / (white - black)` in float | DNG uses the same conceptual mapping to linear reference values, but specifies clipping above 1.0 and permits preserving negative values. darktable's `rawprepare` does no explicit clipping. |
| Crop and pattern phase | Crop origin is added when selecting CFA slot; filter descriptor is shifted | DNG patterns originate at ActiveArea top-left. Both systems require explicit phase rebasing; implementations differ in where they store it. |
| Masked pixels | Crop margins; optional decoder sampling of black areas | DNG `MaskedAreas` is optional metadata for estimating black. darktable does not require masked-pixel measurement for normal processing. |
| White balance | Applied pre-demosaic as per-CFA multiplication | WB is metadata in DNG/LibRaw and optional processing in `fornax`; it must not be applied implicitly. |
| Process state | Module disabled for already-normalized float images | Neither raw buffer nor four-level metadata alone proves a correction has not already happened; explicit state is still needed. |

The comparison is for reconciliation, not a claim that darktable implements
the DNG model as a format library. darktable is a complete RAW developer with
additional image-processing choices.

### 2.8 Licensing

`[verified]` darktable is GPL-3.0 (`LICENSE`; source headers identify GPL v3 or
later). GPLv3 code cannot be copied into a differently licensed Rust crate
without resolving compatibility and distribution obligations. This report
recommends reuse of concepts and independently written formulas, not source
code.

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| Current RAW module order | `src/common/iop_order.c:306-316` |
| Non-reorderable preprocessing constraints | `src/common/iop_order.c:829-838` |
| `rawprepare` fields and defaults | `src/iop/rawprepare.c:46-55,786-804` |
| Decoder supplies black/white/crop metadata | `src/imageio/imageio_rawspeed.cc:246-339` |
| Normalization constants | `src/iop/rawprepare.c:724-747` |
| Correction and FMA formula | `src/iop/rawprepare.c:328-430`; `data/kernels/basic.cl:33-53` |
| No clipping inside `rawprepare` | `src/iop/rawprepare.c:352-366,468-472` |
| CFA slot indexing | `src/iop/rawprepare.c:316-324` |
| Crop-shifted CFA descriptor | `src/iop/rawprepare.c:300-312,368-373` |
| Optional gain map | `src/iop/rawprepare.c:430-468`; `data/kernels/basic.cl:55-97` |
| Temperature/WB multiplication | `src/iop/temperature.c:508-624` |
| Demosaic input and output path | `src/iop/demosaic.c:699-755` |
| Available demosaic methods | `src/iop/demosaic.c:152-172,928-971,1586-1596` |
| "darkroom" is an application view | `src/views/darkroom.c:20` |
| GPL-3.0 license | `agent-work/references/darktable/LICENSE` |

No focused `rawprepare` unit test was found by source inspection. The numerical
formula, CPU/OpenCL parity comments, module-order table, and rawspeed metadata
mapping are the evidence used here. Runtime comparison with a built darktable
was not performed.

## 4. Implications for fornax

`[recommendation]` Use `rawprepare` as evidence for a clear, explicit
sensor-domain normalization stage:

```text
stored sample -> optional linearization -> subtract black -> normalize
              -> explicit clipping policy -> demosaic-ready mosaic
```

`[recommendation]` Keep `sub`, `div`, CFA phase, crop origin, and white point as
typed inputs to preprocessing. Do not collapse them into an image-wide average
or infer them from the cropped buffer.

`[recommendation]` Apply black subtraction using source sensor coordinates and
the correct CFA/color-plane slot. If the input is cropped, preserve the original
sensor origin or rebase both the CFA and black pattern explicitly.

`[recommendation]` Decide clipping and overflow policy separately from
subtraction. darktable's `rawprepare` can retain negative and >1 values; DNG
recommends clipping >1 and permits retaining negative values. `SYN-02` must
resolve this rather than inheriting either behavior implicitly.

`[recommendation]` Represent normalized output with a distinct sample-domain
type or processing-state marker. darktable disables `rawprepare` for selected
normalized float images, but `fornax` should make double application
structurally difficult.

`[recommendation]` Treat optional gain-map, highlight reconstruction, hot-pixel
correction, raw denoise, and chromatic-aberration correction as separate
operations with their own evidence and tests. Do not add them to the initial
black-level implementation.

`[recommendation]` Keep WB, color matrices, tone mapping, denoising (beyond an
explicitly justified RAW-domain operation), sharpening, and display rendering
out of the pipeline.

## 5. Open questions and limitations

- `rawprepare` does not have a dedicated unit-test file in the inspected source.
  Its behavior was established from implementation and comments rather than a
  numerical test suite.
- darktable receives four black/white values from rawspeed. The point at which
  rawspeed reduces a general DNG repeat pattern to those values was not traced;
  this should be treated as decoder-specific behavior.
- `rawprepare` does not clip. The exact downstream clipping point for negative
  values and highlights is spread across later modules and was not treated as
  part of the preprocessing contract.
- Masked-pixel sampling is explicitly described as broken in the rawspeed
  integration at this revision; conclusions about masked-area-derived black
  levels are limited.
- The current `v50_order` is one revision of a configurable pipeline. Optional
  modules and presets can change what executes; the report describes the
  default RAW ordering and module constraints, not every user configuration.
- The relationship of `highlights` to a future `fornax` highlight policy is a
  design decision and remains unresolved.

## 6. References

- darktable repository: `https://github.com/darktable-org/darktable.git`
  at `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`.
  - `src/common/iop_order.c`
  - `src/iop/rawprepare.c`
  - `src/iop/temperature.c`
  - `src/iop/demosaic.c`
  - `src/imageio/imageio_rawspeed.cc`
  - `data/kernels/basic.cl`
  - `src/views/darkroom.c`
  - `LICENSE`
- Local checkout: `agent-work/references/darktable/`.
- darktable license: GPL-3.0; compare with LibRaw/DNG only conceptually.
