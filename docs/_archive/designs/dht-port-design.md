# DHT faithful behavioral port

This archived design note documents the selected DHT faithful behavioral port approach.

## Intent

Make Fornax's `Demosaic::Dht` implementation reproduce LibRaw 0.22.3's
`dht_interpolate()` path for conventional 2x2 RGB Bayer mosaics. The immediate
acceptance target is the native GBRG comparison row: MAE at most 2, P99 at most
64, and no more than 0.1% of samples above 1024 codes.

The current DHT selector delegates to the generic directional kernel. It does
not implement LibRaw's hot-pixel handling, directional distance transforms,
direction refinement, diagonal and horizontal/vertical color reconstruction,
channel bounds, or truncating output conversion.

## Goals

- Port the behavior of LibRaw's `pre_interpolate()` followed by
  `dht_interpolate()` into `fornax-raw-pipeline`.
- Preserve the four-pixel padded working plane, initialized neutral samples,
  directional thresholds, pass order, channel bounds, and truncating output
  conversion.
- Keep the existing public `Demosaic::Dht`, `AnyLazy::Dht`, and `DenseLazy`
  interfaces unchanged.
- Support all four conventional Bayer patterns through phase-relative lookup.
- Add focused stage tests and whole-image comparison coverage.
- Record the pinned source, behavioral boundary, and dual-license obligations.

## Non-goals

- LibRaw camera metadata handling, white balance, color matrices, callbacks,
  post-demosaic processing, shrink, half-size, or four-color RGB modes.
- X-Trans, monochrome, or non-Bayer arrays.
- Production calls into LibRaw or C++ reference code.
- Changes to comparison limits or waivers.

## Anti-goals

- Keeping the generic directional approximation and tuning it toward the
  fixture.
- Using exact output equality as the only test of internal stage order.
- Loosening robust limits, waiving a failing native row, or hand-editing a
  baseline.

## Constraints

- Reference source: LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/dht_demosaic.cpp`.
- The comparison adapter calls `pre_interpolate()` and then
  `dht_interpolate()`.
- The comparison bridge supplies an identity camera-to-RGB matrix and no white
  balance.
- Input and output at the comparison boundary are 16-bit codes.
- LibRaw DHT is dual-licensed LGPL-2.1 or CDDL-1.0. A direct translation can
  remain a derivative work, so the reference note must record the source
  notices and flag redistribution for licensing review.

## Approaches

### Faithful Rust port

Translate the pinned LibRaw path into a dedicated `dht.rs` state machine. Keep
the source's 4-pixel-padded `f32[3]` working plane, direction byte, named
thresholds, stage order, and truncating `u16` packing. Add small stage tests
for hots, direction decisions, green reconstruction, color reconstruction, and
public integration, then verify with the comparison harness.

This is the selected approach because it follows the established DCB, AAHD,
VNG, and LMMSE ports, keeps the backend pure Rust, and makes stage-order
regressions independently testable.

### LibRaw production wrapper

Call LibRaw from the production demosaic stage. This would match by
construction but adds a native decoder dependency to the pure-Rust processing
stage and violates the project's stage separation intent. Rejected.

### Shared directional-kernel expansion

Add a much larger DHT mode to `directional.rs`. DHT mutates a padded float
state through direction discovery, refinement, and multiple in-place color
passes with source-specific thresholds. A selector over incompatible kernels
would hide omissions. Rejected.

## Selected design

`DemosaicDht` remains a `DenseLazy` backend. Its entry point accepts images at
least 3x3, matching the comparison bridge, converts input to 16-bit codes, and
fills a working image of `(width + 8) * (height + 8)` `[f32; 3]` samples with
`0.5`. Measured CFA values are copied to their channels, and the two explicit
green CFA phases are unified as LibRaw's three-color `pre_interpolate()` does.

The internal direction map is a byte per padded sample with the same flags and
bit values as LibRaw: horizontal, vertical, diagonal variants, strong
directions, and hot-pixel state. Direction discovery uses the src DSP's
one-stop luminance distance, including its three repeated squarings of the
horizontal/vertical confidence and 1.4 diagonal threshold.

The implementation follows the reference sequence:

1. Initialize bounds and measured channels; detect isolated hot/dead samples.
2. Build horizontal/vertical directions, refine row parities, then refine all
   columns.
3. Interpolate missing green samples from the selected axis.
4. Build and refine diagonal directions.
5. Reconstruct missing red/blue at non-green sites diagonally.
6. Reconstruct red/blue at green sites along horizontal/vertical directions.
7. Restore hot CFA samples, then truncate the float working plane to 16-bit
   RGB through `DenseLazy`.

Tests operate on direct 16-bit fixtures so thresholds, float ratios, truncation,
and channel bounds remain visible. Whole-image validation uses native GBRG as
the independent oracle and all four patterns as consistency checks.

## What is left to the builder

The implementation plan maps this design to tests and checkpoints. Numeric
mismatches are localized to the earliest stage fixture before any whole-image
tuning. Baseline and generated README changes are accepted only after the
native row passes all robust limits.

## Decision record

- The user requested implementation of `[DHT]`; the faithful Rust port is the
  approach consistent with the current Fornax backend design.
- Fixed LibRaw 0.22.3 behavior is the only production behavior in scope.
- Native GBRG remains the acceptance oracle.
