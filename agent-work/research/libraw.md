# LibRaw

> **Task:** `RES-02`
> **Owner:** research agent
> **Status:** `review` (declaration audit complete; black-level layout remains
> unresolved and acceptance review pending)
> **Last updated:** 2026-10-23
> **Upstream versions:** LibRaw `0.22.2`, from the installed development headers
> at `~/.pixi/envs/base/include/libraw/` and the pinned `libraw-dev == 0.22.2`
> in `pixi.toml`. The exact upstream commit was not available offline.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Relevant fields are documented with type, unit, coordinate convention,
>   sentinel values, and validity conditions, with unknown values marked
>   explicitly.
> - [ ] Black-level repeat-pattern and per-channel semantics are fully
>   established, including sensor-coordinate alignment. **Partial:** the
>   declarations and array size are documented, but the layout is not present in
>   the installed headers and remains an open question.
> - [x] `raw_image` versus processed output is distinguished at the API stage,
>   with the pre/post-black uncertainty marked `[inference]`.
> - [x] A field-to-universal-candidate mapping is provided.
> - [x] Unverified claims are labeled `[inference]`.
> - [x] All six required report sections are complete.

## 1. Scope and questions

Investigate LibRaw decoding behavior, sample semantics, CFA representation,
black/white levels, metadata, and relevant APIs. The questions were:

- How is a RAW file parsed, unpacked, and exposed?
- Does a decoded buffer contain original sensor codes, unpacked samples,
  linearized samples, or processed data?
- How are dimensions, stride, sample type, CFA pattern, and CFA origin
  represented?
- How are black level, white level, saturation, masked pixels, and spatially
  varying levels represented and interpreted?
- How are active area, crop, margins, and orientation handled?
- Which metadata is retained, normalized, derived, or discarded?
- What does the existing `libraw` Rust wrapper expose and hide?

The primary evidence is the installed LibRaw 0.22.2 headers, which declare the
data model, plus the local `crates/libraw` and `crates/libraw-sys` wrapper code.
The upstream implementation source was not available offline; claims about
algorithm behavior that are not visible in declarations are labeled
`[inference]`.

## 2. Findings

### 2.1 Data model and version

`[verified]` The installed header set reports LibRaw `0.22.2`
(`libraw_version.h`, `LIBRAW_MAJOR_VERSION 0`, `LIBRAW_MINOR_VERSION 22`,
`LIBRAW_PATCH_VERSION 2`). The workspace pins `libraw-dev == 0.22.2`
(`pixi.toml`).

`[verified]` `libraw_data_t` aggregates `libraw_image_sizes_t sizes`,
`libraw_iparams_t idata`, `libraw_lensinfo_t lens`, `libraw_colordata_t color`,
`libraw_imgother_t other`, and `libraw_rawdata_t rawdata`
(`libraw_types.h:1113-1131`, fields at lines 1116-1129).

`[verified]` The raw data is owned by LibRaw through `raw_alloc`; the pixel
buffers are aliases into that allocation (`libraw_types.h:995-1008` comments
"really allocated bitmap" and "alias").

### 2.2 Raw buffer variants and sample semantics

`[verified]` `libraw_rawdata_t` declares these buffer variants
(`libraw_types.h:993-1018`):

- `ushort *raw_image` — "alias to single_channel variant" (line 998).
- `ushort (*color4_image)[4]` — "alias to 4-channel variant" (line 1000).
- `ushort (*color3_image)[3]` — "alias to 3-color variant decoded by RawSpeed"
  (line 1002).
- `float *float_image` (line 1004), `float (*float3_image)[3]` (line 1006),
  and `float (*float4_image)[4]` (line 1008).
- `short (*ph1_cblack)[2]` and `short (*ph1_rblack)[2]` for Phase One black
  level data (lines 1011-1012).

`[inference]` `raw_image` holds unpacked sensor codes in unsigned 16-bit
storage. This follows from its `ushort` type, the separate `float` variants,
and the existence of a later `libraw_subtract_black` API. The installed headers
do not state whether values are pre- or post-black-correction at this stage;
this must be confirmed against upstream implementation source or tests.

`[verified]` The local Rust wrapper treats `rawdata.raw_image` as a
single-channel `u16` image and `color4_image`/`float*_image` as RGB/RGBA images
(`crates/libraw/src/data_structure/rawdata.rs:9-15`, `25-114`). This is a
wrapper choice, not proof of LibRaw's full semantics.

### 2.3 Dimensions, stride, active area, and orientation

`[verified]` `libraw_image_sizes_t` declares
(`libraw_types.h:215-225`, fields at lines 217-224):

- `ushort raw_height, raw_width, height, width, top_margin, left_margin`.
- `ushort iheight, iwidth`.
- `unsigned raw_pitch`.
- `double pixel_aspect`.
- `int flip`.
- `int mask[8][4]`.
- `ushort raw_aspect`.
- `libraw_raw_inset_crop_t raw_inset_crops[2]`.

`[inference]` `raw_width`/`raw_height` describe the stored raw frame;
`width`/`height` describe the active image; `top_margin`/`left_margin` locate
the active image inside the raw frame. The field names are descriptive, but the
headers provide no comments confirming the exact inclusive/exclusive bounds.
This must be verified against upstream code before becoming a design contract.

`[verified]` The local wrapper exposes `raw_height`, `raw_width`, `height`,
`width`, `top_margin`, `left_margin`, `raw_pitch`, and `flip`
(`crates/libraw/src/data_structure/image_sizes.rs:32-65`). `flip` is converted
to `ImageSizesFlip` with variants `None`, `Rotate180`, `CCW90`, `CW90`
(`image_sizes.rs:9-14`).

`[verified]` `raw_pitch` is exposed but is not used by
`Rawdata::get_rawdata`; that function receives `raw_width` and `raw_height`
from `ImageSizes` and slices the buffer as a contiguous `width * height` region
(`crates/libraw/src/extension/missing_structures.rs:6-16`,
`crates/libraw/src/data_structure/rawdata.rs:18-114`).

`[inference]` If `raw_pitch` is greater than `raw_width * 2` bytes, the local
wrapper's contiguous slice may include padding or misread rows. This is a
correctness risk, not a verified bug; it depends on LibRaw's allocation.

### 2.4 CFA representation

`[verified]` `libraw_iparams_t` declares `int colors`, `unsigned filters`,
`char xtrans[6][6]`, `char xtrans_abs[6][6]`, and `char cdesc[5]`
(`libraw_types.h:188-208`, fields at lines 200-204).

`[inference]` `filters` encodes the color filter array as a bitmask, and
`cdesc` describes channel names. The installed header does not document the bit
layout, sentinel values, or the relationship between `filters` and
`top_margin`/`left_margin`. This must be verified from upstream source before
being used for CFA-origin logic.

`[verified]` `xtrans` and `xtrans_abs` are 6×6 character tables, described in
the local wrapper documentation as Fuji X-Trans row/column to color mappings,
one relative to the visible area and one relative to the sensor edges
(`crates/libraw/src/data_structure/iparams.rs` accessors; the wrapper docs cite
those two coordinate bases).

`[verified]` The local wrapper exposes `colors`, `filters`, `xtrans`,
`xtrans_abs`, and `cdesc` through `IParams`
(`crates/libraw/src/data_structure/iparams.rs:86-110`). It does not expose a
typed CFA pattern or a CFA origin.

### 2.5 Black level, white level, and saturation

`[verified]` `libraw_colordata_t` declares
(`libraw_types.h:781-826`):

- `unsigned cblack[LIBRAW_CBLACK_SIZE]` (line 783).
- `unsigned black` (line 784).
- `unsigned data_maximum` (line 785).
- `unsigned maximum` (line 786).
- `unsigned linear_max[4]` (line 802).
- `float fmaximum` (line 804).
- `ushort white[8][8]` (line 806).
- `float cam_mul[4]` (line 807), `pre_mul[4]` (line 808),
  `cmatrix[3][4]` (line 809), `ccm[3][4]` (line 810),
  `rgb_cam[3][4]` (line 811), `cam_xyz[4][3]` (line 812).
- `libraw_dng_levels_t dng_levels` (line 826).

`[verified]` `LIBRAW_CBLACK_SIZE` is `4104`
(`libraw_const.h:85`), so `cblack` and `dng_cblack` are fixed-size arrays of
4104 unsigned values.

`[inference]` `black` is a scalar black level and `cblack` carries per-channel
or patterned black-level data. The exact layout of the 4104 entries (channel
count, 2D repeat dimensions, row/column order, and alignment relative to
`top_margin`/`left_margin`) is not documented in the installed header. This is a
critical open question for black-level correction and must be resolved from
upstream source before design.

`[verified]` `libraw_dng_levels_t` declares
(`libraw_types.h:254-270`):

- `unsigned dng_cblack[LIBRAW_CBLACK_SIZE]` (line 257).
- `unsigned dng_black` (line 258).
- `float dng_fcblack[LIBRAW_CBLACK_SIZE]` (line 259).
- `float dng_fblack` (line 260).
- `unsigned dng_whitelevel[4]` (line 261).
- `ushort default_crop[4]` with comment "Origin and size" (line 262).
- `float user_crop[4]` with comment "top-left-bottom-right relative to
  default_crop" (line 263).
- `unsigned preview_colorspace` (line 264).
- `float analogbalance[4]` (line 265), `asshotneutral[4]` (line 266),
  `baseline_exposure` (line 267), `LinearResponseLimit` (line 268).

`[inference]` `maximum` and `linear_max[4]` represent white/saturation levels,
with `linear_max` providing per-channel values. The relationship between
`maximum`, `data_maximum`, `linear_max`, and `dng_whitelevel` is not documented
in the installed header and must be verified.

`[verified]` The local wrapper exposes `libraw_subtract_black()` as a direct
call to the C API (`crates/libraw/src/c_api/auxiliary.rs:27-30`) but exposes no
read accessor for `black`, `cblack`, `maximum`, `linear_max`, or
`dng_levels`.

### 2.6 API sequence and processed output

`[verified]` The public C API declares this sequence
(`libraw.h`):

- `libraw_open_file` (line 91) and `libraw_open_buffer` (line 104).
- `libraw_unpack` (line 113).
- `libraw_subtract_black` (line 119).
- `libraw_raw2image` (line 120).
- `libraw_dcraw_process` (line 152).
- `libraw_dcraw_make_mem_image` (line 154).

`[inference]` `libraw_unpack` populates `rawdata`; `libraw_subtract_black`
modifies raw samples; `libraw_raw2image` and `libraw_dcraw_process` produce the
processed output. The separation is visible in the API names and signatures,
but the exact in-place mutation behavior of `libraw_subtract_black` is not
documented in the installed headers.

`[verified]` The local wrapper exposes `unpack()` and `unpack_thumb()`
(`crates/libraw/src/c_api/data_loading.rs:90-95`), `libraw_subtract_black()`,
`raw2image()`, and `dcraw_process()`
(`crates/libraw/src/c_api/auxiliary.rs:24-60`), and a `ProcessedImage` type
(`crates/libraw/src/data_structure/processed_image.rs:8-54`).

### 2.7 Metadata retained by LibRaw

`[verified]` `libraw_iparams_t` retains make, model, software, normalized
make/model, maker index, raw count, DNG version, Foveon flag, colors, filters,
X-Trans tables, channel description, and XMP pointer/length
(`libraw_types.h:188-208`).

`[verified]` `libraw_colordata_t` retains black/white levels, linear maxima,
white balance multipliers, camera matrices, DNG levels, and camera model
strings (`libraw_types.h:781-826`).

`[verified]` The local wrapper exposes the identity fields, `dng_version`,
`colors`, `filters`, X-Trans tables, and XMP
(`crates/libraw/src/data_structure/iparams.rs:43-122`), but not the color/black
fields.

### 2.8 Field-to-universal-candidate mapping

`[recommendation]` Candidate universal fields and their LibRaw sources:

| Universal concept | Candidate LibRaw field | Status |
| --- | --- | --- |
| Sample buffer | `rawdata.raw_image` / `color4_image` / float variants | Verified declaration; semantics need source confirmation |
| Buffer dimensions | `sizes.raw_width`, `sizes.raw_height` | Verified declaration |
| Active dimensions | `sizes.width`, `sizes.height` | Verified declaration; bounds need confirmation |
| Stride | `sizes.raw_pitch` | Verified declaration; local wrapper ignores it |
| Margins | `sizes.top_margin`, `sizes.left_margin` | Verified declaration; coordinate meaning inferred |
| CFA pattern | `idata.filters`, `idata.xtrans`, `idata.xtrans_abs` | Verified declarations; bit layout undocumented |
| Channel description | `idata.cdesc`, `idata.colors` | Verified declarations |
| Black level | `color.black`, `color.cblack`, `color.dng_levels` | Verified declarations; layout undocumented |
| White/saturation | `color.maximum`, `color.linear_max`, `color.dng_levels.dng_whitelevel` | Verified declarations; precedence unknown |
| White balance | `color.cam_mul`, `color.pre_mul`, `color.dng_levels.asshotneutral` | Verified declarations; not applied by `fornax` |
| Color matrices | `color.cmatrix`, `color.ccm`, `color.rgb_cam`, `color.cam_xyz` | Verified declarations; not applied by `fornax` |
| Orientation | `sizes.flip` | Verified declaration |
| Crop | `color.dng_levels.default_crop`, `user_crop` | Verified declarations; DNG-specific |
| Provenance | `idata.make/model/software`, `dng_version` | Verified declarations |

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| LibRaw version 0.22.2 | `~/.pixi/envs/base/include/libraw/libraw_version.h`; `pixi.toml` |
| Data model aggregation | `libraw_types.h:1113-1131` |
| Raw buffer variants | `libraw_types.h:993-1018` |
| Image sizes and flip | `libraw_types.h:215-225` |
| CFA and identity fields | `libraw_types.h:188-208` |
| Black/white/color fields | `libraw_types.h:781-826` |
| DNG levels | `libraw_types.h:254-270` |
| `LIBRAW_CBLACK_SIZE` | `libraw_const.h:85` |
| API sequence | `libraw.h:91-154` |
| Rust rawdata conversion | `crates/libraw/src/data_structure/rawdata.rs:9-114` |
| Rust image sizes | `crates/libraw/src/data_structure/image_sizes.rs:9-65` |
| Rust iparams | `crates/libraw/src/data_structure/iparams.rs:33-122` |
| Rust C API calls | `crates/libraw/src/c_api/auxiliary.rs:24-60`, `c_api/data_loading.rs:90-95` |
| Rust bindings | `crates/libraw-sys/src/bindings-linux.rs:4841-6771` |

## 4. Implications for fornax

`[recommendation]` The universal representation must carry, or explicitly
reject, every field needed to interpret samples: buffer variant, dimensions,
stride, margins, CFA representation, black level, white level, and
white-balance/color data. Preserving metadata that the current pipeline does not
consume is required by the specification, but applying it is not.

`[recommendation]` Do not adopt LibRaw's `cblack[4104]` array as the universal
black-level type until its layout is verified. A universal type should express
scalar, per-channel, and repeat-pattern black levels explicitly, with a clear
coordinate origin.

`[recommendation]` Treat `raw_image` as sensor codes in the decoder stage. Do
not silently call `libraw_subtract_black` while unpacking; preprocessing must be
an explicit stage with observable state.

`[recommendation]` Preserve `raw_pitch`, `top_margin`, `left_margin`, and
`flip` in the decoder output. The current `Rawdata` conversion loses them.

`[recommendation]` The LibRaw adapter should expose a metadata snapshot
alongside the buffer, or a typed view over `libraw_data_t`, rather than returning
only an `image::ImageBuffer`.

`[recommendation]` Keep LibRaw-specific types behind the adapter. The universal
types should not depend on `libraw_sys` structs or on `image::Luma`.

## 5. Open questions and limitations

- `[verified]` The exact LibRaw upstream commit is not available offline. The
  version is known from the headers and `pixi.toml`, but commit-level citations
  and tests could not be inspected.
- `[inference]` `raw_image` is presumed to contain pre-black-correction sensor
  codes. This must be confirmed from upstream implementation source.
- `[inference]` The layout and coordinate alignment of `cblack` and
  `dng_cblack` are unknown. The installed header gives only the array size.
- `[inference]` The bit layout of `filters`, and whether it is relative to the
  active area or the raw frame, is unknown.
- `[inference]` The precedence among `maximum`, `data_maximum`, `linear_max`,
  and `dng_whitelevel` is unknown.
- `[inference]` The meaning of `mask[8][4]` and `raw_inset_crops[2]` is not
  documented in the installed header and was not investigated further.
- `[verified]` The local wrapper does not expose black/white metadata and
  ignores `raw_pitch`. This is a confirmed gap in `fornax`, independent of
  LibRaw's internal behavior.
- `[verified]` `libraw_subtract_black` is available as a direct C call, but the
  wrapper provides no state marker to detect double application.

## 6. References

- LibRaw 0.22.2 installed headers:
  - `~/.pixi/envs/base/include/libraw/libraw.h`
  - `~/.pixi/envs/base/include/libraw/libraw_types.h`
  - `~/.pixi/envs/base/include/libraw/libraw_const.h`
  - `~/.pixi/envs/base/include/libraw/libraw_version.h`
- LibRaw API declarations cited above: `libraw_open_file`,
  `libraw_open_buffer`, `libraw_unpack`, `libraw_subtract_black`,
  `libraw_raw2image`, `libraw_dcraw_process`,
  `libraw_dcraw_make_mem_image`.
- LibRaw data structures cited above: `libraw_data_t`, `libraw_rawdata_t`,
  `libraw_image_sizes_t`, `libraw_iparams_t`, `libraw_colordata_t`,
  `libraw_dng_levels_t`.
- Local wrapper: `crates/libraw/src/data_structure/{rawdata,image_sizes,iparams,data}.rs`,
  `crates/libraw/src/c_api/{auxiliary,data_loading}.rs`,
  `crates/libraw/src/extension/missing_structures.rs`.
- Generated bindings: `crates/libraw-sys/src/bindings-linux.rs`.
- Version pin: `pixi.toml` (`libraw-dev == 0.22.2`).
- Upstream repository URL: not available offline; record it with a commit
  during acceptance review if network access is restored.
