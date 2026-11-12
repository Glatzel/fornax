# Decoder Golden-Corpus and Vendor Codec Ownership Plan

This plan preserves the completed golden-corpus benchmark work and extends the
same branch and pull request with vendor codec ownership, decoder feature
selection, and proportional benchmark CI.

**Goal:** Keep each vendor stream implementation with its vendor decoder,
restore a vendor-neutral codec contract, let `fornax-decoder` compile only the
requested formats, and make each golden benchmark matrix item compile and run
only the features needed by that input.

**Architecture:** `fornax-codec` owns generic requests, sample buffers,
predictors, byte order, errors, Huffman support, and the generic compression
enum. Renamed vendor crates own private codec enums implementing
`ICodecDecode`. `fornax-decoder` owns the public all-format `Decoder`, with one
optional dependency and feature per vendor. The golden benchmark keeps separate
Forrnax metadata, decode, raw-mosaic, and cropped-mosaic measurements, while
Rawler and FlashRaw expose only decode and raw-mosaic measurements.

## Scope

In scope:

- Hard-rename the format crates to vendor-first names without aliases.
- Move vendor codec implementations, parameters, and tests into those crates.
- Restore `ICodecDecode` and remove compression from `DecodeRequest`.
- Add feature-gated format dependencies and decoder dispatch.
- Make known disabled formats return `DecodeError::Unsupported`.
- Refactor benchmark sample type, reference measurements, and support preflight.
- Add matrix feature mapping and precise changed-file impact groups.
- Update active documentation, manifests, lockfile, examples, and imports.

Out of scope:

- Archived plans or historical records.
- A feature surface on the `fornax-raw` facade.
- Compatibility aliases for old crate names or codec variants.
- Timed benchmark execution during local implementation.
- Changes to the decoder tracker baseline, `TODO.md`, or RAW payload handling.

## Public Interfaces

Hard renames:

| Old | New |
| --- | --- |
| `fornax-arw` | `fornax-raw-sony` |
| `fornax-cr3` | `fornax-raw-canon` |
| `fornax-nef` | `fornax-raw-nikon` |
| `fornax-raf` | `fornax-raw-fujifilm` |
| `fornax-hasselblad` | `fornax-raw-hasselblad` |
| `fornax-rw2` | `fornax-raw-panasonic` |
| `fornax-iiq` | `fornax-raw-phase-one` |

`fornax-dng` remains unchanged.

`fornax-decoder` features are `sony`, `canon`, `nikon`, `fujifilm`,
`hasselblad`, `panasonic`, and `phase-one`. All seven are default features.
DNG is unconditional. `zstd` remains separate and non-default.

The restored contract is:

```rust
pub trait ICodecDecode {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError>;
}
```

`DecodeRequest` has no `compression` field. `DecodeCompression` retains only
Uncompressed, Lossless JPEG, Deflate, LZW, PackBits, Lossy JPEG, JPEG XL, and
Zstandard, and implements `ICodecDecode`. The free `fornax_codec::decode`
dispatcher is removed. Encoding keeps its current `EncodeCompression` behavior.

## Task Checklist

## Task 10: Record the new phase and run the baseline

- [ ] Update this plan and the ledger without discarding completed benchmark
  evidence.
- [ ] Run focused decoder-tracker, benchmark input, matrix, and workspace
  compilation tests from the starting branch.

## Task 11: Hard-rename format crates

- [ ] Use `git mv` for directories and update package names, paths, imports,
  examples, READMEs, workspace dependencies, and lockfile.
- [ ] Do not add aliases.

## Task 12: Restore generic codec ownership and move vendor codecs

- [ ] Add `ICodecDecode`; make the generic compression enum implement it;
  remove compression from decode requests and remove the free dispatcher only
  after all callers migrate.
- [ ] Move Sony ARW1/ARW2/ARW6/Sony-LJPEG, white-level derivation, and LLVC3
  data to `fornax-raw-sony`.
- [ ] Move Canon CRX header/CFA types and decoder to `fornax-raw-canon`.
- [ ] Move Nikon packed, word, striped, and Huffman codecs plus tree
  identifiers to `fornax-raw-nikon`.
- [ ] Move Fujifilm packed/lossless codecs and CFA layout type to
  `fornax-raw-fujifilm`.
- [ ] Move Hasselblad predictor-8 lossless-JPEG handling to
  `fornax-raw-hasselblad`.
- [ ] Move Panasonic V4/V5/V6/V8 and stamped layouts plus V8 tables to
  `fornax-raw-panasonic`.
- [ ] Move Phase One uncompressed, lossless, nonlinear, and Sv2 codecs to
  `fornax-raw-phase-one`.
- [ ] Keep codec enums crate-private and move vendor codec tests with their
  code.
- [ ] Update DNG, TIFF raster callers, and generic codec tests to call
  `ICodecDecode::decode`.

## Task 13: Gate decoder formats and prove feature isolation

- [ ] Make vendor dependencies optional, gate enum variants and every dispatch
  arm, and map disabled known extensions to `DecodeError::Unsupported` with the
  required feature name.
- [ ] Ensure disabled formats never reach Adobe DNG Converter.
- [ ] Add tests for default/all features, DNG-only, every single feature, and
  disabled known formats.
- [ ] Verify normal dependency trees for DNG-only, each vendor feature, and all
  defaults.

## Task 14: Refactor decoder benchmarks

- [ ] Remove `FORNAX_BENCH_BITS`, `BenchParams`, all f64 cases, and combined
  decoder measurements.
- [ ] Keep separate Fornax `decode`, `raw_mosaic`, and `cropped_mosaic` items.
- [ ] Expose only separate Rawler and FlashRaw `decode` and `raw_mosaic` items.
- [ ] Preflight reference decode and raw-mosaic support; skip only typed
  `DecodeError::Unsupported` with a visible case/reference warning.
- [ ] Fail backend errors, malformed output, missing support, and panics.

## Task 15: Map benchmark matrix items to features and changed paths

- [ ] Add one required feature to every matrix row: no feature for DNG, vendor
  features for ARW/CR3/NEF/RAF/3FR/RW2/IIQ.
- [ ] Build each job with `--no-default-features` plus only the matrix feature.
- [ ] Add impact-map tests and planner tests for format filtering, feature
  mapping, and exactly one matrix item per input.
- [ ] Apply precise vendor/Bmff groups; run everything for `fornax-codec`,
  shared crates, benchmark source, manifests, scripts, or workflow changes.

## Task 16: Update active documentation and references

- [ ] Update root guidance, architecture, decoder benchmark/corpus docs, crate
  READMEs, active reference notes, and examples.
- [ ] Search active files for old crate/import names and retired codec feature
  names, allowing them only in archived history.

## Task 17: Validate, review, and hand off

- [ ] Run moved codec tests, generic codec tests, focused feature builds/tests,
  dependency trees, benchmark compilation per feature, matrix tests, golden
  hash checks for all 30 files, and `scripts/agent-validation.sh`.
- [ ] Inspect the complete diff and perform a fresh self-review if no reviewer
  agent is permitted.
- [ ] Commit on the existing branch, push, and update PR #717. Do not merge,
  watch, or close it.

## Files Expected to Change

- Root `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, and active architecture docs.
- `crates/fornax-codec/**`.
- The seven renamed vendor crate directories and their manifests, READMEs,
  source, and tests.
- `crates/fornax-decoder/{Cargo.toml,README.md,src/**,tests/**,benches/**}`.
- `crates/fornax-dng/**`, `crates/fornax-tiff/**`, and callers of changed
  codec APIs.
- `crates/decoder-tracker/scripts/{benchmark_matrix.py,test_benchmark_matrix.py}`.
- `.github/workflows/ci.yml`.
- `docs/decoder/{benchmark.md,corpus.md}`, active decoder references, and this
  plan.

## Validation

```bash
cargo test -p fornax-codec
cargo test -p fornax-raw-sony -p fornax-raw-canon -p fornax-raw-nikon
cargo test -p fornax-raw-fujifilm -p fornax-raw-hasselblad
cargo test -p fornax-raw-panasonic -p fornax-raw-phase-one
cargo test -p fornax-decoder --no-default-features
cargo test -p fornax-decoder --features sony
cargo test -p fornax-decoder
cargo test -p fornax-decoder --all-features
cargo test -p decoder-tracker --lib
python3 -m unittest crates/decoder-tracker/scripts/test_benchmark_matrix.py
python3 crates/decoder-tracker/scripts/test_benchmark_impact.py
cargo bench -p fornax-decoder --bench decoder --no-run
cargo bench -p fornax-decoder --bench decoder --no-default-features \
  --features phase-one --no-run
cargo tree -p fornax-decoder --no-default-features --edges normal
cargo tree -p fornax-decoder --no-default-features --features sony --edges normal
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
```

Timed decoder benchmarks are CI-only. Compile them locally without running the
benchmark bodies.

## Progress

Tasks 10 through 17 are implemented and validated. The work is committed on
the existing branch and is ready for push and pull-request handoff.

## Decisions and Notes

- The completed golden benchmark remains on this branch and PR #717. Its
  previous ledger entries are historical evidence, not open work.
- The earlier combined decode/raw-mosaic benchmark wording is superseded by
  the explicit instruction to keep separate items and give references only
  decode plus raw-mosaic.
- Any `fornax-codec` change conservatively runs every DNG and format benchmark.
  Per-module impact precision is intentionally deferred.
- `fornax-raw` stays a full-feature facade. Reduced builds target
  `fornax-decoder` directly.
- Benchmark reference support is a typed capability preflight. A reference
  that fails supported decode or raw-mosaic with `Unsupported` is skipped with
  a visible warning; all other failures fail the matrix job.
- RAW payloads remain transient and are never cached or uploaded.
- The implementation starts from the latest `origin/main` baseline already
  merged into this branch plus the completed golden benchmark commits.
