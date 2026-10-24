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
        Input["RawImage\nphase-aware"]
        Strategy["DemosaicAlgorithm\nLinear"]
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

Image-rs owns the output allocation and dimensions. `fornax` validates exact
sample counts and finite values before returning the image.

## Strategy interface

`fornax::protocol` contains the phase protocol used to validate the global call
flow:

```rust
pub trait IDemosaic: Send + Sync {
    fn demosaic(
        &self,
        input: &RawImage,
        options: &DemosaicOptions,
    ) -> Result<RgbImageF32, DemosaicError>;
}
```

`fornax::demosaic::DemosaicAlgorithm` is the current selectable algorithm
type. It implements `IDemosaic` and dispatches `Linear` to the in-crate linear
implementation. A later contract task may add the runtime strategy/registry
interface below while preserving the `RawImage` input and interleaved linear
f32 output boundary.

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

1. Bilinear — implemented in `fornax::demosaic::linear`. It preserves the
   known CFA sample and averages available same-color neighbors, including
   reduced neighborhoods at image borders.
2. Malvar–He–Cutler — planned fixed higher-quality kernel with explicit border
   behavior.
3. Edge-aware algorithms — deferred until quality and options are evaluated.

Demosaicing algorithms are implemented inside `fornax::demosaic`. No external
demosaic crate or adapter is part of the target architecture.

## Exclusions

Demosaicing does not perform black correction, normalization, WB, camera color
matrices, highlight reconstruction, gain maps, denoising, sharpening, tone
mapping, or display color conversion.
