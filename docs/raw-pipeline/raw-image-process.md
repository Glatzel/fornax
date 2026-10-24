# Raw Image Processing Flow

This guide describes the proposed processing path from a decoder adapter to a
linear demosaiced image. The normative type and invariant details are in
[DES-001 §3–§9](../../agent-work/design/raw-pipeline/DES-001-universal-raw-representation.md#3-stage-typed-image).

## End-to-end flow

```mermaid
flowchart TD
    File["RAW file\nDNG / ARW / NEF / Canon CR*"]
    Adapter["Selected decoder adapter\nLibRaw / dnglab-rawler / dnc bridge"]
    Decoded["DecodedRaw\nU8 ≤8 bits / U16 9–16 / U32 17–32 / F32"]
    Custom["DecodedRaw after convert_to_f32\nauto f32; optional user operation"]
    Linear["LinearizedRaw\nf32 working buffer\nLUT applied once"]
    Subtract["BlackCorrectedRaw\nf32 samples; f64 scalar arithmetic"]
    Normalize["NormalizedRaw\nunclipped f32"]
    Ready["DemosaicReadyMosaic\nclamped f32 [0,1]"]
    Demosaic["Bayer demosaic\nf32 planar RGB"]
    Done["Linear RGB\nend of pipeline scope"]
    Reject["Structured rejection"]

    File --> Adapter
    Adapter -->|supported Bayer| Decoded
    Adapter -->|unsupported layout / encoding| Reject
    Decoded -->|standard path| Linear
    Decoded -->|"convert_to_f32 → still DecodedRaw"| Custom
    Custom -->|metadata basis valid| Linear
    Custom -->|metadata basis invalid| Reject
    Linear --> Subtract
    Subtract --> Normalize
    Normalize --> Ready
    Ready --> Demosaic
    Demosaic --> Done
```

## Stage contract

| Stage | Payload | Main operation | Required invariant | Failure mode |
| --- | --- | --- | --- | --- |
| Decode / custom | u8/u16/u32/f32 sensor plane | Backend unpack; optional `convert_to_f32` (lossless when representable) and user operation while state remains `Decoded` | Native encoding, geometry, CFA, ActiveArea, levels, provenance, and metadata basis validated | Unsupported encoding/layout or lossy conversion |
| Linearization | f32 plane | Apply declared LUT once, otherwise identity | ActiveArea/CFA geometry unchanged | Missing/invalid table |
| Black subtraction | f32 plane | Select black by active CFA/tile coordinate; subtract once | Black and white levels valid; no clamp yet | Unsupported pattern/delta |
| Normalization | f32 plane | `(sample - black) / (white - black)` with f64 scalar arithmetic | Signed and overflow values remain unclipped | `white <= black` or non-finite |
| Demosaic-ready | f32 plane | Clamp to `[0,1]` and validate finite values | Bayer phase and ActiveArea unchanged | Wrong state/non-finite values |
| Demosaic | f32 planar RGB | Apply selected Bayer algorithm | Planar channel order and border policy explicit | Unsupported algorithm/layout |

The conversion itself is lossless for u8/u16 inputs. It does not prove that
arbitrary custom processing preserves the meaning of file-declared black or
white levels; `Decoded` retains that metadata-basis decision.

## Where preprocessing stops

The required `fornax` pipeline ends after demosaicing. It does not perform:

- white-balance multiplication;
- camera-to-working-space color matrices;
- highlight reconstruction;
- raw denoise, hot-pixel, or chromatic-aberration correction;
- tone mapping or curves;
- display/output color conversion, gamma, denoising, or sharpening.

Those metadata may be preserved, but they must not be applied in this pipeline.

## Related guides

- [Precision by phase](precision.md)
- [Key structures and fields](raw-image-struct.md)
