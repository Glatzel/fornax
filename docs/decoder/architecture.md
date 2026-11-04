# Decoder architecture

## Status

The pure-Rust decoder is implemented in `fornax-core` for the current DNG MVP.
The former standalone `fornax-decoder`, Rawler, Adobe DNG Converter, LibRaw
wrapper, and `libraw-sys` crates are removed from the workspace.

The native decoder reads classic-TIFF DNG files, preserves typed root and raw
IFD metadata, decodes uncompressed and lossless JPEG-92 strips/tiles, and hands
conventional 2x2 RGB Bayer data to the shared pipeline. The writer regenerates
uncompressed DNG output. Source-preserving lossless export remains unavailable
because the current public decoded-data model does not retain container bytes.

## Format order

| Tier | Order | Format plan | Current state |
| --- | ---: | --- | --- |
| 0 | 1 | [DNG](formats/dng.md) | MVP implemented |
| 1 | 2 | [Canon CR3](formats/canon-cr3.md) | Planned |
| 1 | 3 | [Sony ARW](formats/sony-arw.md) | Planned |
| 1 | 4 | [Nikon NEF](formats/nikon-nef.md) | Planned |
| 2 | 5 | [Fujifilm RAF](formats/fuji-raf.md) | Planned |
| 2 | 6 | [Hasselblad 3FR](formats/hasselblad-3fr.md) | Planned |
| 2 | 7 | [Panasonic RW2](formats/panasonic-rw2.md) | Planned |
| 2 | 8 | [Phase One IIQ](formats/phase-one-iiq.md) | Planned |
| 3 | 9 | [Other formats](formats/other.md) | Planned |

## Pipeline interfaces

The decoder boundary is:

```rust
pub trait IDecoder<I, T> {
    fn decode(&self, input: I) -> Result<(MosaicData, Metadata<T>), DecodeError>;
    fn decode_active(&self, input: I)
        -> Result<(DecodedRaw<T>, Metadata<T>), DecodeError>;
}
```

`fornax_core::Decoder` implements this trait for `&Path`. `decode` returns the
full stored mosaic and normalized pipeline metadata. `decode_active` crops to
`Metadata::active_area` and shifts the Bayer phase by the crop origin before
returning `DecodedRaw<T>`.

The preprocessor boundary remains `DecodedRaw<T> -> RawImage<T>`. A decoder
must reject non-2x2 CFA layouts before constructing active pipeline input.

## Module responsibilities

| Module | Responsibility |
| --- | --- |
| `decoder/mod.rs` | `Decoder` type and decoder module ownership |
| `decoder/protocol.rs` | `IDecoder<&Path, T>` integration and DNG-to-pipeline metadata conversion |
| `decoder/dng_decoder/tiff.rs` | TIFF header, IFD, SubIFD, and typed metadata parsing |
| `decoder/dng_decoder/mod.rs` | Raw IFD selection, strip/tile assembly, and uncompressed sample decoding |
| `decoder/dng_decoder/ljpeg.rs` | Restart-free lossless JPEG-92 component decoding |
| `metadata/dng.rs` | `DngTag`, `DngValue`, and deterministic `DngMetadata` map |
| `metadata/mod.rs` | Pipeline metadata, levels, and white balance |
| `writer.rs` | Uncompressed DNG generation and writer options |

## DNG data flow

```text
DNG bytes
  -> classic TIFF directories
  -> full-resolution CFA/LinearRaw IFD
  -> typed root + raw metadata
  -> strip/tile sample decode
  -> MosaicData + DngMetadata
  -> Metadata<T>
  -> optional active-area crop and Bayer phase shift
  -> DecodedRaw<T>
```

`CFAPattern` is interpreted relative to `ActiveArea`. Full-frame and active
patterns therefore differ when the active origin has an odd coordinate.

## Metadata and validation

The decoder preserves supported DNG values in
`BTreeMap<DngTag, DngValue>` while retaining their TIFF value type. Unknown
well-formed tags are retained in the map. Writer serialization regenerates
layout-dependent offset tags and writes the remaining values.

Validation covers:

- TIFF offsets, counts, dimensions, and allocation-size arithmetic;
- raw IFD geometry and full-resolution selection;
- `ActiveArea` and default-crop bounds;
- black-level repeat shape and level values;
- `CFARepeatPatternDim`, `CFAPlaneColor`, `CFALayout`, and `CFAPattern`;
- conventional 2x2 RGB Bayer conversion.

Unsupported compression, sample layouts, and malformed metadata return typed
decode errors rather than panics.

## Writer behavior

- `DngCompression::Uncompressed` regenerates a classic-TIFF single-strip CFA
  DNG from current samples and metadata.
- `DngCompression::LosslessJpeg` is the default option but currently returns
  `DngWriteError::Unsupported` when no source container is available.
- The writer does not silently change sample types, white levels, or CFA tags.

## References and licensing

The Adobe DNG 1.7.1.0 specification and TIFF 6.0 are the primary sources.
Pinned `external/dnglab`/Rawler and `external/libraw` checkouts are behavioral
references only. They are not production dependencies.

- dnglab/Rawler: LGPL-2.1. Do not copy or adapt encoded source without a
  deliberate licence review.
- LibRaw: LGPL-2.1 or CDDL-1.0. Keep it development-only unless a licensed
  derivative is explicitly accepted.
- darktable and RawTherapee comparison sources retain their own GPL terms and
  remain isolated in `compare-demosaic`.

## Testing

Unit tests cover TIFF parsing, checked geometry, strip/tile assembly,
uncompressed decoding, lossless-JPEG component decoding, and selected metadata
conversion. The fixture-backed integration test verifies the accepted DNG,
active-area handoff, representative metadata, uncompressed readback, and the
typed lossless-export limitation.

The full corpus tracker remains a backlog item. Binary RAW fixtures stay in
ignored external storage; only text baselines and metadata are committed.

## Migration

The production migration is complete for the current DNG MVP:

1. `fornax_core::Decoder` is the only production decoder.
2. `fornax`, examples, benchmarks, and `compare-demosaic` use it directly.
3. Rawler, DNC, LibRaw wrappers, generated bindings, and their C/FFI
   dependencies are removed from the workspace.
4. LibRaw, darktable, and RawTherapee remain development-only references for
   demosaic comparison.
5. `fornax-py` remains excluded from the workspace pending its API redesign.

Future format work must preserve the DNG-shaped core model and the conventional
2x2 Bayer handoff unless a separately approved architecture change broadens
that pipeline boundary.
