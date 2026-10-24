# Demosaicing Interface

Demosaicing is the final phase in the required pipeline. It accepts the
f32-only preprocessed `RawImage` and returns linear image-rs RGB f32.

```mermaid
flowchart LR
    subgraph DecodePhase["1 · Decode"]
        Decoded["DecodedRaw"]
    end
    subgraph PreprocessPhase["2 · Preprocess"]
        Ready["RawImage\nf32, [0,1]"]
    end
    subgraph DemosaicPhase["3 · Demosaic"]
        Input["DemosaicInput\nphase-aware"]
        Strategy["Configured strategy\nobject-safe"]
        Output["ImageBuffer<Rgb<f32>>\nlinear output"]
        Input --> Strategy
        Strategy --> Output
    end
    Decoded --> Ready
    Ready --> Input
```

## Input contract

`DemosaicInput` borrows:

- a single-channel f32 mosaic of `width * height` samples;
- dimensions and the validated `BayerLayout`;
- ActiveArea-relative phase information; and
- immutable validated metadata.

Validation rejects:

- incorrect buffer length;
- zero or overflowing dimensions;
- non-finite samples;
- samples outside `[0,1]`; and
- unsupported CFA phases.

The input is immutable. `slot_at_active(row,col)` maps dense active-buffer
coordinates through the original ActiveArea origin before selecting the Bayer
slot.

## Output contract

`RgbImageF32` is
`image::ImageBuffer<image::Rgb<f32>, Vec<f32>>`. It stores
`3 * width * height` interleaved f32 samples:

```text
[R G B][R G B]...
```

Output dimensions equal input dimensions. Values remain linear and are not
implicitly quantized or clipped. Algorithms document overshoot/undershoot
behavior; callers may apply an explicit output policy where appropriate.

Image-rs owns the output allocation and dimensions. Core validates exact
sample counts and finite values before returning the image.

## Strategy interface

`fornax-core` contains the draft phase protocol used to validate the global
call flow:

```rust
pub trait IDemosaic: Send + Sync {
    fn demosaic(
        &self,
        input: &RawImage,
        options: &DemosaicOptions,
    ) -> Result<RgbImageF32, DemosaicError>;
}
```

`IMPL-DEM-001` may refine this into the runtime strategy/registry interface
below while preserving the `RawImage` input and interleaved linear f32 output
boundary.

```rust
pub trait DemosaicStrategy: Send + Sync {
    fn id(&self) -> DemosaicAlgorithmId;
    fn capabilities(&self) -> DemosaicCapabilities;

    fn validate(
        &self,
        input: &DemosaicInput<'_>,
        output: &RgbImageF32,
    ) -> Result<(), DemosaicError>;

    fn process(
        &self,
        input: &DemosaicInput<'_>,
        output: &mut RgbImageF32,
    ) -> Result<DemosaicRun, DemosaicError>;
}
```

The interface is object-safe so one runtime registry can hold independently
compiled algorithms. Algorithm-specific options belong to the configured
strategy object, not the shared input type.

`DemosaicCapabilities` records:

- Bayer phases supported;
- minimum dimensions;
- border policy; and
- algorithm identity.

## Border behavior

Supported descriptions are:

- `Clamp` — out-of-range kernel coordinates clamp to image edges;
- `BilinearFallback` — the algorithm uses bilinear reconstruction near
  borders; and
- `AlgorithmDefined` — the implementation documents and tests its own policy.

The output always has the input dimensions. A strategy requiring an internal
halo gathers it only from available input; no implicit crop or CFA rebase is
allowed.

## Metadata retention

`RawImage::demosaic` returns `(RgbImageF32, Arc<ValidatedMetadata>)`, retaining
source metadata without a wrapper type. It does not apply or discard WB, color
matrices, orientation, crop, or decoder provenance. No per-pixel metadata is
emitted.

## Errors and allocation

`DemosaicError` covers invalid input/output dimensions, non-finite or
out-of-range samples, unsupported phases, too-small images, duplicate/unknown
algorithms, failed strategies, and allocation failure.

The demosaicing boundary checks `3 * width * height` overflow, exact buffer
length, finite values, and output dimensions. Callers may reuse matching
image-rs buffers. Strategies may allocate private scratch buffers; allocation
failure is structured.

## Initial algorithms

1. Bilinear — deterministic baseline and cheapest implementation.
2. Malvar–He–Cutler — fixed higher-quality kernel with explicit border behavior.
3. Edge-aware algorithms — deferred until quality and options are evaluated.

The Rust `demosaic` crate is the initial adapter candidate: map the validated
Bayer phase and configured algorithm, convert its planar f32 result to
image-rs interleaved RGB, and do not expose its CFA/error/algorithm types as
the public `fornax` contract.

## Exclusions

Demosaicing does not perform black correction, normalization, WB, camera color
matrices, highlight reconstruction, gain maps, denoising, sharpening, tone
mapping, or display color conversion.
