# Precision by Phase

Precision has two dimensions: the **container** used to store samples and the
**semantic precision** declared by the source. They are not interchangeable.
This file is the normative precision definition; the field-level contract is in
[Key structures and fields](raw-image-struct.md).

## Precision flow

```mermaid
flowchart TD
    subgraph DecodePhase["1 · Decode"]
        U8["u8 source\n≤8 declared bits"]
        U16["u16 source\n9–16 declared bits"]
        U32["u32 source\n17–32 declared bits"]
        F32S["f32 source\nfloating RAW / DNG"]
        Work["DecodedRaw::convert_to_f32()\nf32; still DecodedRaw\noptional user operation"]
    end

    U8 -->|exact| Work
    U16 -->|exact| Work
    U32 -->|"exact only if ≤24 significant bits"| Work
    F32S -->|no u16 round-trip| Work

    subgraph PreprocessPhase["2 · Preprocess"]
        Preprocess["DecodedRaw::pre_process\nsingle public entry"]
        Linear["private linearize\nf32; exact LUT lookup"]
        Subtract["private subtract_black\nf32 + f64 scalar arithmetic"]
        Normalize["private normalize\nf64 division → f32 storage"]
        Ready["RawImage\nf32, clamped [0,1]"]
    end

    subgraph DemosaicPhase["3 · Demosaic"]
        Demo["Demosaic output\nlinear f32 RGB"]
    end

    Work --> Preprocess
    Preprocess --> Linear
    Linear --> Subtract
    Subtract --> Normalize
    Normalize --> Ready
    Ready --> Demo
```

## Storage rules

| Declared source | Container | Semantic note |
| --- | --- | --- |
| Unsigned integer ≤8 bits | `ImageBuffer<Luma<u8>, Vec<u8>>` | Container width does not rewrite bit depth |
| Unsigned integer 9–16 bits | `ImageBuffer<Luma<u16>, Vec<u16>>` | No implicit left shift |
| Unsigned integer 17–32 bits | `ImageBuffer<Luma<u32>, Vec<u32>>` | Requires backend/source evidence; not every value is exact in f32 |
| Floating DNG / float decoder path | `ImageBuffer<Luma<f32>, Vec<f32>>` | Do not round-trip through u16 |
| Preprocessing | `ImageBuffer<Luma<f32>, Vec<f32>>` | f64 only for scalar arithmetic |
| Demosaic output | planar `f32` | Preserve linear values; do not quantize to integer |

Signed integer samples, integer precision above 32 bits, and f64 source planes
require a separately verified wider path or structured rejection.

## What f32 guarantees

- All 8-bit and 16-bit unsigned integers are represented exactly.
- Fractional normalized values and negative underflow are representable.
- Multiplying/dividing by scalar metadata does not require quantizing back to
  an integer between operations.
- f32 has 24 bits of significand precision; it is not a universal replacement
  for integer values above 24 significant bits or arbitrary f64 source data.
  A u32 conversion is therefore accepted only when declared precision is ≤24
  bits; wider integer values need a separately designed wider path.
- f64 is used for black/white arithmetic, not as the image buffer type. f64
  buffers double memory and bandwidth without recovering information already
  represented in the f32 preprocessing input.

## Custom processing rule

Converting decoded u8/u16/u32 data to f32 for custom processing is opt-in and
keeps the value as `DecodedRaw` with an f32 plane. The custom
operation must preserve the black/white semantics established by the decoder;
standard preprocessing always uses those validated metadata levels.

## Related guides

- [Raw image processing flow](raw-image-process.md)
- [Key structures and fields](raw-image-struct.md)
