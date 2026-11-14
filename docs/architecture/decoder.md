# Decoder Architecture

This document explains decoder dispatch, shared interfaces, and per-format data flow.

## Status

The default production decoder is the pure-Rust implementation in
`fornax-decoder`. It dispatches DNG containers to `fornax-dng`, Canon CR3/CRM
containers to `fornax-raw-canon`, Sony ARW/NRW containers to `fornax-raw-sony`, Nikon NEF
containers to `fornax-raw-nikon`, Fujifilm RAF containers to `fornax-raw-fujifilm`, Hasselblad
3FR containers to `fornax-raw-hasselblad`, Panasonic RW2 containers to
`fornax-raw-panasonic`, and Phase One/Leaf IIQ containers to `fornax-raw-phase-one`.
`fornax-raw-core` owns the shared decoder protocol and pipeline types but no
decoder implementation.

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
| 0 | 1 | [DNG](../decoder/formats/dng.md) | Default decoder implemented |
| 1 | 2 | [Canon CR3](../decoder/formats/canon-cr3.md) | Implemented for audited lossless CRX; C-RAW rejected |
| 1 | 3 | [Sony ARW](../decoder/formats/sony-arw.md) | Implemented for uncompressed, ARW1, ARW2, ARW6, and tiled LJPEG |
| 1 | 4 | [Nikon NEF](../decoder/formats/nikon-nef.md) | Partially implemented for standard uncompressed/packed modes |
| 2 | 5 | [Fujifilm RAF](../decoder/formats/fuji-raf.md) | Implemented for uncompressed and block-lossless Bayer/X-Trans |
| 2 | 6 | [Hasselblad 3FR](../decoder/formats/hasselblad-3fr.md) | Implemented for accepted uncompressed and predictor-8 samples |
| 2 | 7 | [Panasonic RW2](../decoder/formats/panasonic-rw2.md) | Implemented for single-shot V4/V5/V6/V8 and stamped 12-bit Bayer |
| 2 | 8 | [Phase One IIQ](../decoder/formats/phase-one-iiq.md) | Implemented for modes 0, 3, 5, 6, and 8 |
| 3 | 9 | [Other formats](../decoder/formats/other-formats.md) | Planned |

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
| `fornax-bmff/src` | Bounded ISO-BMFF boxes, FullBoxes, UUID payloads, and sample tables |
| `fornax-tiff/src` | TIFF tags, typed field constants, values, directories, checked parsing, and classic-TIFF writing |
| `fornax-codec/src` | Format-neutral requests, sample buffers, chunk assembly, predictors, Huffman and byte-order support, `ICodecDecode`, generic codecs, and supported codec encoders |
| `fornax-dng/src/metadata.rs` | DNG-only `DngTag` values and deterministic JSON over `Directory` |
| `fornax-dng/src/constants.rs` | DNG-owned compression and photometric interpretation values |
| `fornax-raw-sony/src` | Sony TIFF selection, private metadata, ARW codec ownership, CFA layout mapping, and DNG metadata mapping |
| `fornax-raw-canon/src` | Canon BMFF/track selection, CMT/CTMD metadata, CMP1/IAD1 geometry, CRX codec ownership, and CFA types |
| `fornax-raw-nikon/src` | Nikon TIFF selection, type-2 maker notes, packed/word/striped/Huffman codec ownership, metadata mapping, and Bayer handoff |
| `fornax-raw-fujifilm/src` | RAF pointer/tag parsing, Fuji IFD selection, packed/lossless and CFA-layout codec ownership, metadata mapping, and Bayer-only handoff |
| `fornax-raw-hasselblad/src` | 3FR TIFF/SubIFD selection, camera profiles, uncompressed and predictor-8 decoding, metadata mapping, and Bayer handoff |
| `fornax-raw-phase-one/src` | Phase One private-IFD parsing, IIQ codec ownership, sensor correction, metadata mapping, and Bayer handoff |
| `fornax-raw-panasonic/src` | Panasonic TIFF layout selection, V4-V8 and stamped codec ownership, V8 tables, metadata mapping, DNG round-trip, and Bayer handoff |
| `fornax-decoder/src/lib.rs` | Default DNG, Canon CR3/CRM, Sony ARW/NRW, Nikon NEF, Fujifilm RAF, Hasselblad 3FR, Phase One/Leaf IIQ, and Panasonic RW2 dispatch and `IDecoder` implementation |
| `fornax-decoder-dnc/src/wrapper.rs` | Adobe DNG Converter invocation and Rawler delegation |
| `fornax-decoder-rawler/src/lib.rs` | Rawler decoding and metadata adaptation |
| `fornax-dng/src/dng_decoder/mod.rs` | Raw IFD selection, DNG-to-codec request mapping, transforms, and active crop |
| `fornax-dng/src/dng_decoder/process_metadata.rs` | DNG geometry, levels, CFA, and white-balance conversion |
| `fornax-dng/src/writer.rs` | Uncompressed and lossless DNG generation |
| `crates/fornax-decoder/benches/decoder.rs` | Golden-corpus decoder-method benchmarks for default, Rawler, and FlashRaw adapters |
| `decoder-tracker/src` | Manual raw.pixls.us manifest, isolated decoder runs, shard merge, and baseline diff |

`fornax-codec` keeps only format-neutral codec features. Vendor stream
implementations live in their vendor crates and implement the private
`ICodecDecode` codec values there. `fornax-decoder` exposes `sony`, `canon`,
`nikon`, `fujifilm`, `hasselblad`, `panasonic`, and `phase-one`; all are
default-enabled, while DNG remains unconditional and `zstd` remains separate.
A known vendor extension whose feature is disabled returns a typed unsupported
error naming the feature instead of falling through to DNC conversion.

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

## Fujifilm RAF data flow

```text
RAF bytes
  -> header pointer blocks
  -> type-less RAF tag table
  -> embedded Fuji TIFF/IFD
  -> full-resolution Bayer/X-Trans CFA
  -> packed, word, or Fujifilm-lossless decode
  -> RawMosaic
  -> mapped DNG directory
  -> CroppedMosaic<T> for 2x2 Bayer only
```

Uncompressed 12-bit packed and 14/16-bit word storage are supported alongside
Fujifilm 12/14/16-bit block-lossless compression. Explicit GFX profiles supply
2x2 RGGB and active-area borders. X-Trans is derived from the 36-plane RAF tag
and remains available through raw mosaic and DNG export. Its process metadata
and cropped mosaic accessors return a typed unsupported-CFA error.

Lossy block compression, SuperCCD rotation/staggering, FinePix DBP, unknown
Bayer models, and unknown compression families remain outside the implemented
tier.

## Hasselblad 3FR data flow

```text
3FR bytes
  -> classic little-endian TIFF
  -> root previews and full-resolution CFA SubIFD
  -> explicit camera profile when CFA tags are absent
  -> uncompressed words or predictor-8 lossless JPEG
  -> RawMosaic
  -> mapped DNG directory
  -> CroppedMosaic<T> for 2x2 Bayer only
```

Compression 1 requires exact little-endian 16-bit word storage. Compression 7
uses two leading Huffman lengths per pixel pair, two attached differences,
per-row predictor resets, the 3FR `MSB32` four-byte order, and class-16 payload
consumption. Compression-7 strip byte counts are recorded inconsistently in
accepted files and therefore do not bound the payload; the checked strip offset
does.

Accepted camera models use explicit RGGB profiles. X1D and X1D II remove
inactive sensor borders. FFF, merged/multi-shot, tiled, unknown tagless models,
and unknown compression families remain outside the implemented tier.

## Canon CR3/CRM data flow

```text
CR3/CRM bytes
  -> ISO-BMFF boxes and CRX brand validation
  -> CCTP-selected CRAW track and sample tables
  -> CMT1-CMT4 TIFF metadata
  -> CMP1/IAD1 geometry and CFA
  -> CTMD ColorData
  -> lossless CRX entropy decode and plane placement
  -> RawMosaic::U16
  -> CroppedMosaic<T>
```

The first raw sample is selected. Preview, JPEG, HEIF, and auxiliary tracks are
never selected. The accepted CRX scope is pinned by CMP1 version `0x0100`,
four planes, encoding type zero, zero transform levels, and MDAT header size
`0x70`; every other combination is typed unsupported. IAD1 inclusive endpoints
are converted once, CMP1 overshoot is clamped, and a valid crop outside the
active area becomes the active area.

## Phase One/Leaf IIQ data flow

```text
IIQ bytes
  -> classic TIFF directories
  -> private header at offset 8
  -> private maker-note IFD and relative offsets
  -> per-row IIQ codec chunks
  -> RawMosaic U16
  -> black, curve, quadrant, defect, and flat-field correction
  -> mapped DNG directory and ProcessMetadata
  -> CroppedMosaic<T>
```

Modes 0, 3, 5, 6, and 8 are accepted. Corrected samples export black level zero
and white level 65535. Explicit profiles cover the conventional 2x2 Phase One
and Leaf models in the pinned corpus. Legacy modes and unknown profiles remain
typed unsupported paths.
## Panasonic RW2 data flow

```text
RW2 bytes
  -> classic TIFF magic 85 directories
  -> first primary RawDataOffset IFD, else StripOffsets
  -> raw-format/compression validation
  -> V4/V5/V6/V8 or stamped 12-bit decode
  -> RawMosaic
  -> mapped DNG directory + active-area process metadata
  -> CroppedMosaic<T>
```

Single-shot 2x2 Bayer V4, V5, V6, and V8 layouts are supported. Border and
crop coordinates are exclusive at right/bottom, CFA is shifted into the active
reference frame, and the V4 black offset is applied before DNG metadata
conversion. Multi-shot markers, high-resolution composites, V7, unknown
versions/compression, and unknown CFAs return typed unsupported errors before
sample decoding. See the [RW2 format plan](../decoder/formats/panasonic-rw2.md) and
[Panasonic reference note](../reference/decoders/panasonic-rw2.md).

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

X-Trans metadata preserves a 6x6 `CFARepeatPatternDim` and all 36
`CFAPattern` entries through raw decoding and DNG writing. The RAF handoff
rejects that layout before preprocessing rather than reducing it to Bayer.

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
and `cropped_mosaic` for the default decoder and benchmark-only Rawler and
FlashRaw adapters across the 30-file tier-0/1 lossless golden corpus. Fornax metadata accessors
remain diagnostic-only. It is separate from the preprocessor and demosaic
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
lossless-JPEG component decoding, Sony packing and row transforms, Fujifilm
packed/block-lossless decoding, Hasselblad predictor-8 ordering and bit order,
Rawler tag conversion, and DNC argument construction. Benchmark changes should
at minimum compile with
`cargo bench -p fornax-decoder --no-run`. Fixture-backed DNG, Sony, Nikon,
Fujifilm, Hasselblad, Rawler, and DNC checks depend on the ignored external RAW
fixtures and platform tools available in the development environment.

`decoder-tracker` holds the manual full-corpus regression workflow. The
raw.pixls.us submodule is checked out as pointer metadata only; selected LFS
payloads remain transient, and only text manifests, baselines, and metadata are
committed.

## Migration

The decoder migration is complete for the current DNG, Sony ARW/NRW, Nikon NEF,
Fujifilm RAF, and Hasselblad 3FR stages:

1. `fornax-decoder::Decoder` is the default pure-Rust DNG, Sony ARW/NRW,
   Nikon NEF, Fujifilm RAF, and Hasselblad 3FR facade.
2. Rawler and Adobe DNG Converter adapters are separate workspace crates.
3. `fornax-raw-core` contains shared interfaces and types, not decoder
   implementations.
4. `fornax-decoder` owns decoder benchmarks.
5. LibRaw, darktable, and RawTherapee remain comparison-only references.
6. `fornax-py` remains excluded from the workspace pending its API redesign.

Future format work must preserve the decoder trait and the conventional 2x2
Bayer handoff unless a separately approved architecture change broadens the
pipeline boundary.
