# Universal RAW Representation Guides

These documents explain the proposed `fornax` RAW image model at three
different levels:

- [Raw image processing flow](raw-image-process.md) describes how a decoded
  file moves through custom processing, preprocessing, and demosaicing.
- [Precision by phase](precision.md) explains when data uses u8, u16, f32, or
  f64 and what each choice guarantees.
- [Key structures and fields](raw-image-struct.md) maps the proposed Rust types,
  fields, units, and invariants.

The normative design is
[`DES-001-universal-raw-representation.md`](../../agent-work/design/raw-pipeline/DES-001-universal-raw-representation.md).
These guides explain that design; they do not introduce additional behavior.

## Reading map

```mermaid
flowchart LR
    Decode["DecodedRaw\nnative u8/u16/u32/f32"]
    Convert["Optional DecodedRaw\nf32 custom processing"]
    Pre["Preprocess\nf32 buffer + f64 scalars"]
    Demo["Demosaic\nf32 input/output"]
    Output["Linear RGB\nf32"]

    Decode --> Pre
    Decode -->|"convert_to_f32"| Convert
    Convert --> Pre
    Pre --> Demo
    Demo --> Output
```

Use [Precision by phase](precision.md) when deciding a sample type and
[Key structures and fields](raw-image-struct.md) when implementing or reviewing
the data model.
