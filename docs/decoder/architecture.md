# Decoder architecture

## Status

The default production decoder is the pure-Rust implementation in
`fornax-decoder`. It dispatches DNG containers to `fornax-dng` and Sony ARW/NRW
containers to `fornax-arw`. `fornax-raw-core` owns the shared decoder protocol and
pipeline types but no decoder implementation.

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
| 1 | 3 | [Sony ARW](formats/sony-arw.md) | Implemented for uncompressed, ARW1, ARW2, ARW6, and tiled LJPEG |
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
    type Metadata;

    fn decode(input: I) -> Result<Self, DecodeError>;
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError>;
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError>;
    fn dng_metadata(&self) -> Result<Self::Metadata, DecodeError>;
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
| `fornax-raw-core/src/protocol.rs` | `IDecoder` and cross-stage processor traits |
| `fornax-raw-core/src/primitive.rs` | `RawMosaic`, `CroppedMosaic`, `NormalizedMosaic`, and image aliases |
| `fornax-raw-core/src/metadata/process.rs` | `ProcessMetadata<T>` and `Levels<T>` |
| `fornax-tiff/src` | TIFF tags, typed field constants, values, directories, checked parsing, and classic-TIFF writing |
| `fornax-codec/src` | Format-neutral sample decoding, chunk assembly, predictors, and supported codec encoders |
| `fornax-dng/src/metadata.rs` | DNG-only `DngTag` values and deterministic JSON over `Directory` |
| `fornax-dng/src/constants.rs` | DNG-owned compression and photometric interpretation values |
| `fornax-arw/src` | Sony TIFF selection, private metadata, Sony codec dispatch, CFA decoding, and DNG metadata mapping |
| `fornax-decoder/src/lib.rs` | Default DNG and Sony ARW/NRW dispatch and `IDecoder` implementation |
| `fornax-decoder-dnc/src/wrapper.rs` | Adobe DNG Converter invocation and Rawler delegation |
| `fornax-decoder-rawler/src/lib.rs` | Rawler decoding and metadata adaptation |
| `fornax-dng/src/dng_decoder/mod.rs` | Raw IFD selection, DNG-to-codec request mapping, transforms, and active crop |
| `fornax-dng/src/dng_decoder/process_metadata.rs` | DNG geometry, levels, CFA, and white-balance conversion |
| `fornax-dng/src/writer.rs` | Uncompressed and lossless DNG generation |
| `crates/fornax-decoder/benches/decoder.rs` | Decoder-method benchmarks for default and Rawler adapters |
| `decoder-tracker/src` | Manual raw.pixls.us manifest, isolated decoder runs, shard merge, and baseline diff |

Codec availability is controlled by Cargo features. The TIFF raster resolver
distinguishes unknown compression IDs from known IDs whose required codec
feature is disabled, and the DNG adapter preserves that distinction in its
typed decode errors.

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

## Sony ARW/NRW data flow

```text
ARW/NRW bytes
  -> TIFF directories
  -> full-resolution CFA IFD
  -> decrypted Sony private IFD
  -> Sony compression decode
  -> RawMosaic
  -> mapped DNG directory + process metadata
  -> CroppedMosaic<T>
```

The accepted bitstream families are uncompressed 12-bit packed and 14/16-bit
word samples, ARW1, ARW2, ARW6, and single-component tiled LJPEG. Sony's
four-component LJPEG layout is transformed into its two Bayer output rows.
Reduced-resolution YCbCr/`LinearRaw`, multi-sample ARQ, unknown compression,
and non-Bayer CFA layouts remain outside the Bayer handoff.

## Metadata and validation

The DNG decoder preserves supported values in a `fornax_tiff::Directory`, the
ordered `BTreeMap<Tag, Value>` that retains each TIFF field type. `TiffTag`
names TIFF 6.0 and TIFF/EP fields, `DngTag` names DNG-only fields, and
`fornax_dng::constants` owns DNG-specific field values; all convert to or index
the dynamic `Tag`/numeric domains without narrowing unknown IDs. Unknown
well-formed tags remain in the map. The Rawler adapter synthesizes and forwards
the same directory model.

Validation covers:

- TIFF offsets, counts, dimensions, and allocation-size arithmetic;
- raw IFD geometry and full-resolution selection;
- `ActiveArea` and default-crop bounds;
- black-level repeat shape and level values;
- `CFARepeatPatternDim`, `CFAPlaneColor`, `CFALayout`, and `CFAPattern`;
- conventional 2x2 RGB Bayer conversion.

Unsupported sample layouts and malformed metadata return typed decode errors
rather than panics. Unknown compression identifiers are invalid input, while a
recognized codec can still reject an unsupported payload layout.

Self-assigned TIFF JPEG XL identifier `50002` and DNG 1.7 identifier `52546`
map to the single-channel JPEG XL decoder. Three-sample `LinearRaw` payloads,
including compression `1`, `7`, and `34892` examples in the corpus, are
rejected before codec dispatch because the raw mosaic contract accepts one
sample per pixel.

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

`fornax-decoder` compiles one benchmark target covering `decode`, `raw_mosaic`,
`cropped_mosaic`, `dng_metadata`, and `process_metadata` for the default DNG
and Rawler decoders. It is separate from the preprocessor and demosaic
benchmarks in their stage crates.

## Writer behavior

- `DngCompression::Uncompressed` regenerates a classic-TIFF single-strip CFA
  DNG from current `RawMosaic` samples and a `Directory`.
- `DngCompression::LosslessJpeg` writes 256x256 tiled JPEG-92 for U8/U16
  mosaics and tiled lossless Deflate for U32/F32 mosaics.
- `DngCompression::Zstd` is implemented with pure-Rust `structured-zstd` and is
  always available.
- Structural and chunk-layout tags are regenerated; consumed frontend tags are
  omitted so they are not applied twice.
- `TiffDirectory` in `fornax-tiff` owns shared entry accumulation and writer
  finalization; DNG code owns which structural fields to emit and omit.
- The writer does not silently change sample types, levels, or CFA tags.
- Output remains regenerated rather than byte-identical because the public
  model does not own the source container or auxiliary directories.

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

Unit tests cover TIFF read-overflow checks, private-IFD offset correction,
lossless-JPEG component decoding, Sony packing and row transforms, Rawler tag
conversion, and DNC argument construction. Benchmark changes should at minimum
compile with `cargo bench -p fornax-decoder --no-run`. Fixture-backed DNG, Sony,
Rawler, and DNC checks depend on the ignored external RAW fixtures and platform
tools available in the development environment.

`decoder-tracker` holds the manual full-corpus regression workflow. The
raw.pixls.us submodule is checked out as pointer metadata only; selected LFS
payloads remain transient, and only text manifests, baselines, and metadata are
committed.

## Migration

The decoder migration is complete for the current DNG and Sony ARW/NRW stages:

1. `fornax-decoder::Decoder` is the default pure-Rust DNG and Sony ARW/NRW
   decoder facade.
2. Rawler and Adobe DNG Converter adapters are separate workspace crates.
3. `fornax-raw-core` contains shared interfaces and types, not decoder
   implementations.
4. `fornax-decoder` owns decoder benchmarks.
5. LibRaw, darktable, and RawTherapee remain comparison-only references.
6. `fornax-py` remains excluded from the workspace pending its API redesign.

Future format work must preserve the decoder trait and the conventional 2x2
Bayer handoff unless a separately approved architecture change broadens the
pipeline boundary.
