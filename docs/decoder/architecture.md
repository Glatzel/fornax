# Decoder architecture

## Status

The default production decoder is the pure-Rust DNG implementation in
`fornax-decoder`. `fornax-core` owns the shared decoder protocol and pipeline
types but no decoder implementation.

Rawler and Adobe DNG Converter adapters are also workspace crates:

- `fornax-decoder-rawler` exposes Rawler-decoded samples and metadata through
  `IDecoder`.
- `fornax-decoder-dnc` converts a non-DNG input with Adobe DNG Converter, then
  delegates to the Rawler decoder.

The active preprocess/demosaic handoff remains conventional 2x2 RGB Bayer-only.
The decoder adapters may retain broader format information until the handoff,
but unsupported CFA layouts must return typed errors instead of producing
pipeline input.

## Format order

| Tier | Order | Format plan | Current state |
| --- | ---: | --- | --- |
| 0 | 1 | [DNG](formats/dng.md) | Default decoder implemented |
| 1 | 2 | [Canon CR3](formats/canon-cr3.md) | Planned |
| 1 | 3 | [Sony ARW](formats/sony-arw.md) | Planned |
| 1 | 4 | [Nikon NEF](formats/nikon-nef.md) | Planned |
| 2 | 5 | [Fujifilm RAF](formats/fuji-raf.md) | Planned |
| 2 | 6 | [Hasselblad 3FR](formats/hasselblad-3fr.md) | Planned |
| 2 | 7 | [Panasonic RW2](formats/panasonic-rw2.md) | Planned |
| 2 | 8 | [Phase One IIQ](formats/phase-one-iiq.md) | Planned |
| 3 | 9 | [Other formats](formats/other.md) | Planned |

Rawler may decode some of these formats as an alternate development or
application decoder. That does not make the format a committed native decoder
or expand the conventional Bayer pipeline boundary.

## Pipeline interfaces

The decoder boundary is:

```rust
pub trait IDecoder<I, T> {
    fn decode(input: I) -> Result<Self, DecodeError>;
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError>;
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError>;
    fn dng_metadata(&self) -> Result<DngMetadata, DecodeError>;
    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError>;
}
```

`decode` owns input and parser state in the returned decoder value. The four
accessors let callers request storage-native samples, type-converted active
mosaic input, preserved DNG tags, or normalized process metadata without
exposing decoder internals.

`cropped_mosaic<T>` converts `RawMosaic` samples to `T` and crops to
`ProcessMetadata::active_area`. The returned `CroppedMosaic<T>` feeds
`IPreprocessor<T>`, which returns `NormalizedMosaic<T>`. A decoder must reject
non-2x2 CFA layouts before constructing active pipeline input.

## Module responsibilities

| Crate/module | Responsibility |
| --- | --- |
| `fornax-core/src/protocol.rs` | `IDecoder` and cross-stage processor traits |
| `fornax-core/src/primitive.rs` | `RawMosaic`, `CroppedMosaic`, `NormalizedMosaic`, and image aliases |
| `fornax-core/src/metadata/process.rs` | `ProcessMetadata<T>` and `Levels<T>` |
| `fornax-core/src/metadata/dng.rs` | `DngTag`, `DngValue`, and deterministic `DngMetadata` |
| `fornax-decoder/src/lib.rs` | Default DNG decoder and `IDecoder` implementation |
| `fornax-decoder-dnc/src/wrapper.rs` | Adobe DNG Converter invocation and Rawler delegation |
| `fornax-decoder-rawler/src/lib.rs` | Rawler decoding and metadata adaptation |
| `fornax-decoder/src/dng_decoder/tiff.rs` | TIFF header, IFD, SubIFD, and typed metadata parsing |
| `fornax-decoder/src/dng_decoder/mod.rs` | Raw IFD selection, strip/tile assembly, sample decoding, and active crop |
| `fornax-decoder/src/dng_decoder/ljpeg.rs` | Restart-free lossless JPEG-92 component decoding |
| `fornax-decoder/src/dng_decoder/process_metadata.rs` | DNG geometry, levels, CFA, and white-balance conversion |
| `fornax-core/src/writer.rs` | Uncompressed DNG generation and writer options |
| `bench-decoder/benches/decoder.rs` | Decoder-method benchmarks for DNG and Rawler adapters |

## DNG data flow

```text
DNG bytes
  -> TIFF directories
  -> full-resolution CFA/LinearRaw IFD
  -> typed root + raw metadata
  -> strip/tile sample decode
  -> RawMosaic
  -> CroppedMosaic<T>
  -> ProcessMetadata<T>
  -> Preprocessor
```

`ProcessMetadata.active_area` is relative to the full frame. The Bayer pattern
in process metadata is relative to the cropped mosaic, so callers do not shift
the pattern again. `default_crop` remains relative to the active area and is
not applied automatically.

## Metadata and validation

The DNG decoder preserves supported values in
`BTreeMap<DngTag, DngValue>` while retaining their TIFF value type. Unknown
well-formed tags remain in the map. The Rawler adapter converts Rawler's DNG
override tags into the same typed model.

Validation covers:

- TIFF offsets, counts, dimensions, and allocation-size arithmetic;
- raw IFD geometry and full-resolution selection;
- `ActiveArea` and default-crop bounds;
- black-level repeat shape and level values;
- `CFARepeatPatternDim`, `CFAPlaneColor`, `CFALayout`, and `CFAPattern`;
- conventional 2x2 RGB Bayer conversion.

Unsupported compression, sample layouts, and malformed metadata return typed
decode errors rather than panics.

## Alternates and tooling

`RawlerDecoder<T>` requires a conventional RGB 2x2 CFA before returning
`cropped_mosaic`, but exposes its decoded raw mosaic and process metadata
through the shared trait. Its `dng_metadata` accessor returns the typed tag map
available from Rawler; it is not a claim that every original container tag was
preserved.

`DncDecoder<T>` invokes the platform Adobe DNG Converter, then delegates all
decoder accessors to `RawlerDecoder<T>`. The facade exports DNC only on Windows
and macOS; tests and documentation must not make the DNC path a requirement on
other platforms or on machines without that executable.

`bench-decoder` compiles one benchmark target covering `decode`, `raw_mosaic`,
`cropped_mosaic`, `dng_metadata`, and `process_metadata` for the default DNG
and Rawler decoders. It is separate from the preprocessor and demosaic
benchmarks in their stage crates.

## Writer behavior

- `DngCompression::Uncompressed` regenerates a classic-TIFF single-strip CFA
  DNG from current `RawMosaic` samples and `DngMetadata`.
- `DngCompression::LosslessJpeg` is the default option but currently returns
  `DngWriteError::Unsupported` when no source container is available.
- The writer does not silently change sample types, white levels, or CFA tags.

## References and licensing

The Adobe DNG 1.7.1.0 specification and TIFF 6.0 are the primary sources.
Pinned `external/dnglab`/Rawler and `external/libraw` checkouts are behavioral
references.

- dnglab/Rawler: LGPL-2.1. Do not copy or adapt encoded source without a
  deliberate licence review.
- LibRaw: LGPL-2.1 or CDDL-1.0. It remains comparison-only unless a licensed
  production integration is explicitly accepted.
- darktable and RawTherapee retain their own GPL terms and remain isolated in
  `compare-demosaic`.

## Testing

Unit tests cover TIFF read-overflow checks, lossless-JPEG component decoding,
Rawler tag conversion, and DNC argument construction. Benchmark changes should
at minimum compile with `cargo bench -p bench-decoder --no-run`. Fixture-backed
DNG, Rawler, and DNC checks depend on the ignored external RAW fixtures and
platform tools available in the development environment.

The full corpus tracker remains a backlog item. Binary RAW fixtures stay in
ignored external storage; only text baselines and metadata are committed.

## Migration

The decoder migration is complete for the current DNG MVP:

1. `fornax-decoder::Decoder<T>` is the default pure-Rust DNG decoder.
2. Rawler and Adobe DNG Converter adapters are separate workspace crates.
3. `fornax-core` contains shared interfaces and types, not decoder
   implementations.
4. `bench-decoder` owns decoder benchmarks.
5. LibRaw, darktable, and RawTherapee remain comparison-only references.
6. `fornax-py` remains excluded from the workspace pending its API redesign.

Future format work must preserve the decoder trait and the conventional 2x2
Bayer handoff unless a separately approved architecture change broadens the
pipeline boundary.
