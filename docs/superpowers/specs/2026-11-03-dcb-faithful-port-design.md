# DCB faithful behavioral port

## Intent

Make Fornax's `Demosaic::Dcb` implementation reproduce LibRaw 0.22.3's fixed
DCB configuration, `dcb(0, 1)`, for normal 2x2 RGB Bayer mosaics. The immediate
acceptance target is the native GBRG comparison row: MAE at most 2, P99 at most
64, and no more than 0.1% of samples above 1024 codes.

The current DCB selector delegates to the generic directional kernel. It uses
an edge-weight exponent of two and a two-pixel chroma radius, but it does not
implement LibRaw's directional decisions, Nyquist correction, direction maps,
mixed-direction green corrections, refinement, or final full-color
reconstruction. The DCB row is consequently a whole-frame failure rather than
a border or normalization issue.

## Goals

- Port the behavior of LibRaw's `pre_interpolate()` followed by
  `dcb(iterations = 0, dcb_enhance = 1)` into the `fornax-postprocessor`
  crate.
- Preserve LibRaw's six-pixel border interpolation, stage order, integer
  clipping, and explicit `ushort` conversion points.
- Keep the existing public `Demosaic::Dcb`, `AnyLazy::Dcb`, and `DenseLazy`
  interfaces unchanged.
- Support all four conventional Bayer patterns through phase-relative
  indexing, while using native GBRG as the independent acceptance row.
- Add stage-level tests that fail for omitted or reordered DCB stages.
- Record the pinned reference, behavioral differences, and BSD-3-Clause
  attribution obligations in `docs/references/dcb.md`.

## Non-goals

- FBDD denoising or LibRaw's denoising variants.
- Configurable DCB iteration counts or disabled color enhancement.
- LibRaw camera metadata handling, white balance, color matrices,
  post-demosaic processing, callbacks, or shrink/half-size modes.
- X-Trans, four-color Bayer, monochrome, or generic filter-array support.
- Changes to comparison limits or waivers for the native DCB row.

## Anti-goals

- Calling LibRaw from the production demosaic stage merely to obtain matching
  output.
- Replacing the DCB selector with another approximation of the shared
  directional kernel.
- Re-clamping measured CFA samples to the input after stages where LibRaw
  intentionally permits those values to change.
- Accepting a result based only on MAE when P99 or the high-error fraction
  still fails.

## Constraints

- Reference source: LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/dcb_demosaic.cpp#L849`.
- The comparison adapter calls `pre_interpolate()` and then `dcb(0, 1)`;
  Fornax must match that exact path.
- The stage starts with a normalized active Bayer mosaic and emits normalized
  RGB. The comparison harness converts to 16-bit codes before demosaicing and
  back to the sample type afterward.
- LibRaw's DCB implementation carries a BSD-3-Clause notice from the DCB/FBDD
  author. The port must retain the applicable attribution and license notice.
- The repository workflow requires one branch, worktree, pull request, and
  task plan for the selected `[DCB]` backlog item.

## Approaches

### Faithful Rust port

Translate the fixed LibRaw path into a dedicated `dcb.rs` implementation using
full-frame `u16[4]` state and `f32[3]` horizontal/vertical candidate buffers.
Keep the CFA phase calculations, helper order, clipping, truncating conversions,
and output packing close to the reference. Add small hand-checkable tests for
each behaviorally significant stage and use the comparison harness for
whole-image verification.

This is the selected approach. It keeps the production backend independent and
inspectable, matches the project's integer-first convention used by the VNG
port, and makes the required stage-order behavior testable.

### Production LibRaw wrapper

Expose the comparison shim's LibRaw call through production code. This would
match by construction, but it adds a native processing dependency to a
pure-Rust stage, embeds a second decoder library in the production pipeline,
and bypasses the intended Fornax implementation boundary. This approach is
rejected.

### Shared directional-kernel expansion

Add DCB-specific branches to `directional.rs`. This reduces duplicated setup,
but DCB mutates full-frame state through many distinct in-place passes and uses
different border, clipping, and conversion semantics. The shared abstraction
would become a large selector over incompatible kernels, making omissions
harder to detect. This approach is rejected.

## Selected design

`DemosaicDcb` remains a `DenseLazy` backend. Its lazy entry point accepts
images large enough for DCB's deepest seven-pixel neighborhood and rejects
smaller inputs with `DemosaicError::InvalidInput`. The implementation starts
from `buffer::codes` so integer operations see the same 16-bit values as the
reference adapter, then returns through `buffer::dense_from_codes`.

The internal image state is an RGB-plus-direction array, `[[u16; 4]]`, matching
LibRaw's image layout. The direction map uses channel 3. Two full-frame
`[[f32; 3]]` buffers hold the horizontal and vertical candidate
reconstructions. Because the public output has three channels, channel 3 is
discarded after the final stage.

The implementation follows the reference sequence:

1. Copy measured CFA samples into their channel and run six-pixel border
   interpolation.
2. Build horizontal and vertical green candidates and their red/blue color
   candidates.
3. Select the primary green direction with `dcb_decide`.
4. Save the current red/blue values, run the post-decision color pass and
   green smoothing.
5. Run `dcb_correction2`, then the four map/correction passes in reference
   order.
6. Restore saved red/blue values, run `dcb_color`, refinement, and
   `dcb_color_full`.
7. Pack channels 0 through 2 and report the result through `DenseLazy`.

Helpers operate on explicit width/height and pixel indices, with one shared CFA
channel function derived from `BayerPattern`. Bounds are established by the
same loop ranges as LibRaw rather than by clamping coordinates at image edges.
Where libraw uses `CLIP(int(expression))`, Fornax uses a helper that truncates
to `i32` and clamps to `0..=u16::MAX`; where it converts a floating result
directly to `ushort`, tests pin the intended truncation behavior.

## Validation strategy

Tests are added before replacing the generic backend:

- A two-direction fixture distinguishes horizontal and vertical green
  candidates and the decision result.
- A Nyquist-pattern fixture detects omitted or reordered Nyquist passes.
- A local-edge fixture exercises map/correction mixing.
- An enhancement fixture reaches refinement and full-color reconstruction.
- Small synthetic Bayer fixtures cover all four patterns, uniform channel
  values, six-pixel border behavior, and minimum dimensions.

Whole-image validation uses the native GBRG DCB comparison along with all four
cropped pattern rows. The task is complete only when the native row passes all
three numeric limits, the focused and workspace test commands pass, formatting
and Clippy results are recorded, snapshots and the canonical baseline are
reviewed, and generated README content matches the accepted results.

## What is left to the builder

The implementation plan must map these design decisions to exact source
functions, file changes, test fixtures, and command checkpoints. During
implementation, numeric discrepancies should first be localized to the earliest
stage fixture rather than repaired by changing thresholds or weakening tests.

## Decision record

- The user selected the faithful Rust port over a production LibRaw wrapper
  and over further expansion of the shared directional kernel.
- Fixed `dcb(0, 1)` behavior is the only production behavior in scope.
- The existing public demosaic interface remains stable.
- Native GBRG remains the acceptance oracle; the other patterns provide
  coverage but use one-pixel crops in the comparison harness.
