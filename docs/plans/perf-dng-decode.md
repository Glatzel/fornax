# DNG Decode Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce Fornax DNG raw-decode time to within about 1.5x Rawler on the
integer benchmark cases without changing lossless decode semantics.

**Architecture:** Establish output-equality and end-to-end timing guardrails
first, then optimize the measured hot paths in independently mergeable layers:
LJPEG entropy decode, TIFF source/metadata ownership, uncompressed sample
unpacking, and Deflate sample reconstruction. The existing general LJPEG path
remains the fallback, and filesystem mapping remains outside the container
codec crates.

**Tech Stack:** Rust 2024, `image`, `rayon`, Criterion, Rawler/FlashRaw only as
dev-dependencies, and optionally `memmap2` behind a default desktop feature for
read-only source mapping.

**Spec:** The investigation report supplied in the 2026-11-14 task thread. The
measured baseline and accepted conclusions are reproduced below so this plan
is executable without chat context.

**Investigation evidence:**
[DNG decode performance reference notes](../reference/decoders/dng-performance.md).

## Global Constraints

- Implement pure Rust only. Do not introduce project-authored FFI.
- Keep the current general LJPEG decoder as the fallback for every unsupported
  fast-path shape.
- Keep the optimization scope lossless. Do not add lossy JPEG, lossy DCT,
  JPEG XL, Zstandard, or new decode formats.
- Preserve crate ownership: `fornax-tiff` is the generic TIFF container,
  `fornax-codec` owns codecs, and `fornax-dng` owns DNG metadata and policy.
- Do not put DNG/raw-specific logic in `fornax-tiff`; generic lookup/source
  traits are allowed.
- New dependencies require a concrete measured justification and feature-gating
  where reasonable. The only planned dependency is `memmap2`.
- Declare any new command-line tool in `pixi.toml`. No new external tool is
  planned.
- Rawler and FlashRaw remain benchmark/test-only dependencies.
- Do not commit RAW payloads, host-dependent timing baselines, or generated
  benchmark results.
- Every implementation PR must be green on its own and include before/after
  single-file benchmark results in its description.
- Do not watch, poll, merge, or close a pull request.

## Scope

In scope:

- Three-way raw-mosaic equality for every benchmark input.
- A separate Fornax `decode + raw_mosaic` headline benchmark item.
- A thin-LTO/codegen-units-1 profile check applied equally to all decoders.
- The Blackmagic row-packing correctness bug.
- LJPEG common-case entropy decoding.
- TIFF source ownership and removal of duplicate source/root copies.
- Borrowed root/raw metadata lookup during DNG decode.
- Compact representation of large TIFF byte payloads.
- Uncompressed 12/14/16-bit sample unpacking.
- Deflate predictor and float conversion fusion.

Out of scope:

- SIMD or target-feature-specific kernels.
- Compiler flag changes before the Task 0 experiment justifies them.
- Replacing the Deflate backend.
- Multi-thread scheduling changes.
- Lossy JPEG/DCT, JPEG XL, Zstandard, or vendor formats.
- Cropping, demosaicing, normalization, color, or output encoding.
- A public API for mapping arbitrary paths inside `fornax-tiff`.
- Re-encoding or rewriting DNG files as part of decode.

## Review Focus

- A mapped source must not create dangling slices or expose partially parsed
  state when the file changes.
- The fused LJPEG fast path must remain bit-identical to the general path at
  component, row, first-MCU, and point-transform boundaries.
- JPEG rows that contain multiple TIFF tile rows must map without transposing
  or off-by-one writes.
- Borrowed metadata lookup must preserve DNG merge precedence: the raw IFD
  overrides the root IFD.
- Packed, compressed float, scratch-buffer, and endian paths must preserve exact
  U16/F32 sample bits; no approximate comparison is acceptable.

## Investigation Baseline

Release-mode, single-thread end-to-end results from the investigation:

| File | Variant | Fornax | Rawler | Flashraw | Fornax / Rawler |
| --- | --- | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | U16 strip | 145.4 ms | 48.7 ms | 19.6 ms | 2.98x |
| Sigma fp 4K | 12-bit strip | 66.3 ms | 11.3 ms | 10.5 ms | 5.86x |
| 5D3 LJPEG | U16 tile | 459.2 ms | 262.3 ms | 101.6 ms | 1.75x |
| Pentax K-5 | 14-bit LJPEG strip | 358.3 ms | 155.1 ms | 88.5 ms | 2.31x |
| iPhone SE | U16 LJPEG tile | 185.3 ms | 79.6 ms | 43.3 ms | 2.33x |
| Pixel 8 Pro | U16 LJPEG tile | 1189.6 ms | 565.5 ms | 200.3 ms | 2.10x |
| 5D3 HDR | FP16 predictor Deflate | 324.7 ms | 449.6 ms | 205.2 ms | 0.72x |
| 5D3 HDR | FP32 predictor Deflate | 646.3 ms | 802.3 ms | 338.4 ms | 0.81x |

The target applies to integer DNG paths. The float paths already beat Rawler
and must not regress materially while the Deflate pipeline improves.

The primary measured causes were:

- LJPEG: about 95% of Fornax’s 5D3 LJPEG cycles were in
  `decode_scan_into`, `add_prediction`, and `BitReader::peek`; Fornax executed
  about 5.2x FlashRaw’s instructions and incurred about 20x its branch misses.
- Source/metadata: Fornax spent 115 ms before pixel decode on the 48 MB
  uncompressed file, versus 0.25 ms in FlashRaw `open_path`.
- Pixel 8: `dng_metadata()` cloning cost about 103 ms because a large private
  value expands to millions of individual `Value` entries.
- Uncompressed: 12/14-bit decode used scalar bit extraction plus a `Vec<u32>`
  staging pass.
- Deflate: inflate, whole-tile predictor reversal, and typed conversion were
  three separate passes with per-tile allocation.
- `target-cpu=native`, SIMD, and a zlib backend swap were neutral and are
  excluded.

## API and Design Decisions

### TIFF source ownership

Add a generic source enum in `fornax-tiff`:

```rust
#[derive(Clone)]
pub enum TiffSource {
    Owned(std::sync::Arc<Vec<u8>>),
    #[cfg(feature = "mmap")]
    Mapped(std::sync::Arc<memmap2::Mmap>),
}

impl TiffSource {
    pub fn from_vec(bytes: Vec<u8>) -> Self;
    pub fn from_slice(bytes: &[u8]) -> Self;
    pub fn as_slice(&self) -> &[u8];
}
```

`fornax-raw-decoder` opens DNG files and constructs `TiffSource`. It does not
move path handling into `fornax-tiff`. `memmap2` is justified because both
reference decoders map files, Fornax currently reads and copies the full source
twice, and the measured fixed cost dominates uncompressed DNG decoding.

### Root directory

Remove the duplicated `ParsedTiff::root` storage. Keep the directory in
`directories[0]` and expose:

```rust
impl ParsedTiff {
    pub fn root(&self) -> &Directory;
}
```

### Generic directory lookup

Add a container-neutral lookup trait in `fornax-tiff`:

```rust
pub trait DirectoryLookup {
    fn get(&self, tag: &Tag) -> Option<&Value>;
    fn contains_key(&self, tag: &Tag) -> bool;
}
```

Implement it for `Directory`. `RasterLayout` and `RasterRequest` gain generic
constructors over `DirectoryLookup`; existing `Directory` constructors remain
as wrappers. DNG defines a crate-private `DngDirectory<'a>` overlay whose
`get` consults the selected raw IFD first and the root IFD second.

### Compact large byte values

Add `Value::Bytes(Vec<u8>)`. The parser uses it for TIFF BYTE/UNDEFINED fields
with more than 64 elements; small CFA/pattern fields remain
`Value::List(Vec<Value::Byte>)` for source compatibility.

## Task Checklist

- [x] Task 0: Add equality/headline/profile guardrails.
- [x] Task 1: Fix Blackmagic packed-row LJPEG as an independent correctness
  change.
- [ ] Task 2: Add the fused common-case LJPEG decoder.
- [x] Task 3: Add non-copying TIFF source storage and remove the duplicated
  root directory.
- [x] Task 4: Use borrowed DNG metadata during decode.
- [ ] Task 5: Store large private TIFF byte payloads compactly.
- [ ] Task 6: Add direct uncompressed sample unpacking.
- [ ] Task 7: Fuse Deflate predictor and typed conversion.

---

## Task 0: Guardrails and Fair Baseline

**Files:**

- Create: `crates/fornax-raw-decoder/tests/decoder_equivalence.rs`
- Create: `crates/fornax-raw-decoder/tests/support/mosaic_compare.rs`
- Modify: `crates/fornax-raw-decoder/Cargo.toml`
- Modify: `crates/fornax-raw-decoder/benches/decoder.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/decoder/benchmark.md`

**Interfaces:**

- Consumes: the existing benchmark adapters and golden-input selection.
- Produces:

```rust
pub(crate) enum CanonicalMosaic {
    U8 { width: u32, height: u32, samples: Vec<u8> },
    U16 { width: u32, height: u32, samples: Vec<u16> },
    U32 { width: u32, height: u32, samples: Vec<u32> },
    F32 { width: u32, height: u32, samples: Vec<u32> },
}

pub(crate) fn canonical_mosaic(mosaic: &RawMosaic) -> CanonicalMosaic;
pub(crate) fn assert_mosaic_eq(
    case: &str,
    reference_name: &str,
    expected: &CanonicalMosaic,
    actual: &CanonicalMosaic,
);
```

- Produces benchmark item `fornax/end_to_end`.

- [x] **Step 1: Write the failing equality test**

Add an ignored integration test:

```rust
#[test]
#[ignore = "requires materialized decoder-tracker golden corpus"]
fn every_dng_benchmark_input_matches_across_decoders() {
    // Enumerate the same selected inputs as the benchmark.
    // Decode Fornax, Rawler, and FlashRaw.
    // Compare native storage type, dimensions, and exact sample bits.
}
```

The helper compares `RawMosaic::U8`, `U16`, `U32`, and `F32`; F32 comparison
uses `to_bits()`. A mismatch reports case, decoder, width/height, and sample
index. Typed unsupported results fail the test rather than silently skipping a
benchmarked DNG input. The equality gate is DNG-only because this performance
program targets DNG and the reference adapters do not guarantee byte-identical
raw semantics for every non-DNG vendor format.

- [x] **Step 2: Run the test and verify the current failure**

Run:

```bash
FORNAX_BENCH_RAW='external/raw.pixls.us/<case>' \
cargo test -p fornax-raw-decoder --test decoder_equivalence -- \
  --ignored --nocapture
```

Expected before implementation: unresolved items and missing helper modules.

- [x] **Step 3: Implement the canonical comparison**

Add `tests/support/mosaic_compare.rs`, include it from the integration test via
`#[path = "support/mosaic_compare.rs"]`, include it from the benchmark via
`#[path = "../tests/support/mosaic_compare.rs"]`, and panic on the first
mismatch. Include the three decoded outputs in the test setup once; do not
decode once per pairwise comparison.

- [x] **Step 4: Integrate equality into CI and local runs**

Add a matrix-job step before the Criterion command:

```bash
FORNAX_BENCH_RAW="$FORNAX_BENCH_RAW" \
cargo test -p fornax-raw-decoder --test decoder_equivalence -- \
  --ignored --nocapture
```

Document the all-files local command:

```bash
RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
cargo test -p fornax-raw-decoder --test decoder_equivalence -- \
  --ignored --nocapture
```

- [x] **Step 5: Add the end-to-end benchmark item**

In `bench_fornax`, add:

```rust
group.bench_function("fornax/end_to_end", |b| {
    b.iter(|| {
        let decoder = Decoder::decode(black_box(path))
            .unwrap_or_else(|error| decoder_error(case, "fornax", "decode", error));
        decoder
            .raw_mosaic()
            .unwrap_or_else(|error| decoder_error(case, "fornax", "raw_mosaic", error))
    });
});
```

Keep `fornax/decode`, `fornax/raw_mosaic`, `cropped_mosaic`, and metadata
diagnostics unchanged. The headline ratio is `fornax/end_to_end` versus the
reference `decode` item.

- [x] **Step 6: Make explicit decoder selection survive CI**

Change `selected_decoders()` so an explicit `FORNAX_BENCH_DECODER` always wins.
Only default to `fornax` when CI has not selected decoders.

- [x] **Step 7: Run the thin-LTO/codegen-units experiment**

Compile and measure the same representative cases with all three decoders and
identical flags:

```bash
RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
FORNAX_BENCH_DECODER=fornax,rawler,flashraw \
CARGO_PROFILE_BENCH_LTO=thin \
CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
cargo bench -p fornax-raw-decoder --bench decoder -- \
  'DNG__' --sample-size 20 --warm-up-time 1 --measurement-time 3
```

Run the same command without the two `CARGO_PROFILE_BENCH_*` variables. Put a
five-file before/after table in the PR description. If rankings or any
near-1.5x conclusion changes by more than 5%, add the same settings under
`[profile.bench]`; otherwise record the result and leave the default profile
unchanged. Never apply special flags to only one decoder.

- [x] **Step 8: Verify and commit**

Run:

```bash
cargo test -p fornax-raw-decoder --test decoder_equivalence --no-run
cargo bench -p fornax-raw-decoder --bench decoder --no-run
scripts/agent-validation.sh
```

Expected: the equality test compiles and the focused commands pass; the
equality test remains ignored by default. Record any unrelated workspace-wide
baseline failures separately instead of expanding this task.

Task 0 verification note: focused decoder tests, the full DNG equality gate,
benchmark compilation, and strict clippy for `fornax-raw-decoder` pass. The
workspace validation command is currently blocked by pre-existing clippy
findings in `fornax-dng/src/dng_decoder/transforms.rs` and
`fornax-raw-pipeline/src/demosaic/dht.rs`, both outside this task's diff.

```bash
git add crates/fornax-raw-decoder .github/workflows/ci.yml \
  docs/decoder/benchmark.md
git commit -m "test: add decoder performance guardrails"
```

**Expected gain:** No direct speedup. This task establishes the fairness and
correctness gate for every later item.

**Risks and rollback:** Three-way equality may expose existing semantic
differences unrelated to performance. Resolve those before accepting the
guardrail; do not weaken exact comparison. Revert the single guardrail commit
if CI cost is unacceptable.

---

## Task 1: Fix Blackmagic Packed-Row Lossless JPEG

This is a correctness fix, not part of the performance target.

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/tests/decode_contract.rs`
- Modify: `crates/decoder-tracker/golden/selected-manifest.csv`
- Modify: `crates/decoder-tracker/golden/decode-hashes.csv`
- Modify: `docs/decoder/benchmark.md`
- Modify: `docs/reference/decoders/dng-lossless-jpeg.md`

**Interfaces:**

- Consumes: existing `decode_into(...)` and `decode_interleaved(...)`.
- Produces:

```rust
fn reshape_scan_into(
    decoded_width: usize,
    decoded_height: usize,
    expected_width: usize,
    expected_height: usize,
    decoded: &[u16],
    output: &mut [u16],
) -> Result<(), String>;
```

- [x] **Step 1: Add a synthetic failing row-packing test**

Encode a 4x1 synthetic LJPEG scan whose linear samples are
`[1, 2, 3, 4]`. Decode it as a 2x2 tile and assert:

```rust
assert_eq!(tile, [1, 2, 3, 4]);
```

Also add a mismatched-total test that must remain a typed invalid-input error.

- [x] **Step 2: Run the test and verify the current dimension rejection**

Run:

```bash
cargo test -p fornax-codec decode_contract packed_row -- --nocapture
```

Expected: failure containing the current frame/tile dimension mismatch.

- [x] **Step 3: Implement row folding**

The Blackmagic corpus payload is a valid 12-bit one-component SOF3 stream with
frame geometry 2064x1088 and TIFF tile geometry 1032x2176. It packs two TIFF
rows per JPEG row; total sample count is unchanged. Accept a mismatch only when
the decoded width is a positive integer multiple of `expected_width`, the
decoded sample count equals the expected tile sample count, and linear
reshaping is therefore lossless. Compact each decoded row into consecutive
expected-width rows. Keep all other dimension mismatches invalid.

- [x] **Step 4: Add the corpus regression**

Materialize:

```text
Blackmagic/Blackmagic Pocket Cinema Camera 4k/BMPCC_4k_DCI_lossless.dng
```

Decode it through `fornax-raw-decoder`, assert 4128x2176 RGGB U16 output, and
run the Fornax-versus-Rawler equality check. FlashRaw precision-scales this
12-bit output and is not byte-equivalent, so keep the file out of the shared
three-way golden set and use a focused corpus regression instead.

The selected manifest and existing hashes remain unchanged:

```bash
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

- [x] **Step 5: Verify and commit**

Run:

```bash
cargo test -p fornax-codec
cargo test -p fornax-raw-decoder
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
```

```bash
git add crates/fornax-codec crates/decoder-tracker docs
git commit -m "fix: decode packed-row Blackmagic lossless JPEG"
```

**Expected gain:** Correct decode for the affected Blackmagic file. No timing
commitment.

Task 1 verification note: synthetic row-packing tests pass, the Blackmagic
corpus file decodes as 4128x2176 RGGB U16 and matches Rawler bit-for-bit,
`fornax-codec` tests and package clippy pass, and the unchanged 30-row golden
hash check passes. Full workspace validation remains blocked by the unrelated
clippy findings recorded under Task 0.

**Risks and rollback:** Row folding could mask corrupt dimensions. Require
exact total sample equality and add mismatch tests. Revert this independent PR
without affecting performance work.

---

## Task 2: Fused Common-Case LJPEG Decoder

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/tests/decode_contract.rs`
- Modify: `crates/fornax-codec/tests/huffman.rs`
- Modify: `crates/fornax-codec/README.md`
- Modify: `docs/reference/decoders/dng-lossless-jpeg.md`

**Interfaces:**

- Consumes: Task 1 row-folding semantics.
- Produces private types/functions:

```rust
#[derive(Clone, Copy)]
struct FastHuffmanEntry {
    consumed: u8,
    symbol: u8,
    difference: i16,
    flags: u8,
}

struct FastBitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    cache: u64,
    buffered_bits: u32,
    ended: bool,
}

fn decode_scan_rows(
    entropy: &[u8],
    header: &Frame,
    tables: &[Option<FastHuffmanTable>; 4],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String>;
```

- [x] **Step 1: Add an equivalence oracle**

Keep the current general scanner callable from tests as
`decode_scan_into_reference`. Add tests that encode and decode both 1- and
2-component predictor-1 streams and compare fast or general output bit-for-bit
for complete tiles, one-row tiles, odd widths, and the Blackmagic packed-row
case.

- [ ] **Step 2: Run the tests and verify the fast path is absent**

Run:

```bash
cargo test -p fornax-codec ljpeg_fast
```

Expected: missing fast-path symbols.

- [ ] **Step 3: Implement the left-aligned bit reader**

Use a 64-bit cache whose next bit is the most significant valid bit.
- Refill from eight bytes when possible.
- Handle `0xff 0x00` byte stuffing and marker termination.
- Provide `fill_to(bits)`, `peek_no_fill(bits)`, and `skip_no_fill(bits)`.
- Leave the existing byte-at-a-time reader as the general fallback.

- [x] **Step 4: Implement a 13-bit full-difference Huffman LUT**

Use 8192 entries. Precompute complete differences when code length plus
amplitude length fits 13 bits. Retain canonical max-code/min-code vectors for
long-code fallback, including the DNG 16-bit special case.

- [x] **Step 5: Implement predictor-1 row fusion**

Specialize one and two components. Keep predictors in local registers, decode
one row at a time, and write directly to the destination row when the TIFF tile
is one JPEG row high. For packed-row tiles, use Task 1’s expected-width mapping
and a reusable row buffer. Dispatch to the general scanner for predictors 2–7,
more than two components, restart markers, or unsupported sampling.

- [ ] **Step 6: Remove per-tile whole-frame staging**

Replace the `decode_interleaved -> Vec<u16> -> copy_from_slice` path for
fast-path tiles with row callbacks into the final parallel band. Keep the
generic allocation/copy path unchanged as fallback.

- [x] **Step 7: Prove equivalence on the full DNG corpus**

Run Task 0’s three-way equality test on every selected DNG. Run the old and new
LJPEG scanners over every Fornax-decodable LJPEG DNG in the selected corpus and
compare exact U16 bytes.

- [x] **Step 8: Benchmark acceptance**

Run the end-to-end item on 5D3 tiled LJPEG, Pentax striped LJPEG, iPhone SE,
and Pixel 8. Acceptance:

- `fornax/end_to_end <= 1.5 * rawler/decode` for each case.
- No regression on the generated 16-bit 5D3 strip case.
- Record before/after absolute times and ratios in the PR.

Use:

```bash
RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
FORNAX_BENCH_DECODER=fornax,rawler,flashraw \
FORNAX_BENCH_RAW='external/raw.pixls.us/<dng>' \
cargo bench -p fornax-raw-decoder --bench decoder -- \
  'end_to_end|rawler/decode' --sample-size 30 --warm-up-time 1 \
  --measurement-time 3
```

- [x] **Step 9: Verify and commit**

Run:

```bash
cargo test -p fornax-codec
cargo test -p fornax-raw-decoder
scripts/agent-validation.sh
```

```bash
git add crates/fornax-codec docs/reference/decoders/dng-lossless-jpeg.md
git commit -m "perf: fuse common lossless JPEG decoding"
```

**Expected gain:** The LJPEG path is about 80–90% of the integer gap. Bringing
the 1.75–2.31x Rawler ratios below 1.5 should save roughly 20–35% on affected
files and substantially more relative to FlashRaw.

Task 2 progress note: the 13-bit completed-difference lookup and predictor-1
row fusion are implemented and verified against the general scanner. Current
single-thread Fornax/Rawler ratios are 1.02x (5D3 LJPEG), 1.17x (iPhone SE),
1.49x (Pixel 8), and 1.52x (Pentax). The left-aligned reader and direct
destination staging remain open before Task 2 is complete.

CodSpeed's simulated run reported Sony A1 `raw_mosaic` and `cropped_mosaic`
regressions. Single-thread walltime did not reproduce them: against `d83021b`,
the golden Sony A1 file measured 838.6 ms and 1.04 s on the base versus
669.3 ms and 865.7 ms on the implementation. The CodSpeed report also marked
101 benchmarks as system-call-dominated, which its simulation instrument
excludes.

**Risks and rollback:** Invalid-code handling, odd tile widths, and predictor
state are the main risks. Keep dispatch narrow and the general fallback
available. Revert the task PR if any exact-equality check fails; no data-format
or public API migration blocks rollback.

---

## Task 3: Non-Copying TIFF Source and Single Root Directory

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/fornax-tiff/Cargo.toml`
- Modify: `crates/fornax-tiff/src/lib.rs`
- Modify: `crates/fornax-tiff/src/parser.rs`
- Modify: `crates/fornax-tiff/tests/parser.rs`
- Modify: `crates/fornax-raw-decoder/Cargo.toml`
- Modify: `crates/fornax-raw-decoder/src/lib.rs`
- Modify: all `ParsedTiff.root` callers in DNG and vendor crates
- Modify: DNG/vendor tests that assert `parsed.root`

**Interfaces:**

- Consumes: `TiffSource` and `ParsedTiff::root()` designs above.
- Produces:

```rust
pub fn parse(source: &[u8]) -> Result<ParsedTiff, TiffError>;
pub fn parse_source(source: TiffSource) -> Result<ParsedTiff, TiffError>;
```

- [x] **Step 1: Add source/root behavioral tests**

Test that `parse` and `parse_source(TiffSource::from_vec(bytes))` produce
identical directories and source bytes, that `root()` points at directory zero,
and that no root clone exists.

- [x] **Step 2: Run the tests and verify the APIs are absent**

Run:

```bash
cargo test -p fornax-tiff parse_source
cargo test -p fornax-tiff parses_classic
```

Expected: missing symbols or old public field.

- [x] **Step 3: Add `memmap2` with a feature gate**

Add workspace dependency `memmap2 = "0.9.12"` and:

```toml
[features]
mmap = ["dep:memmap2"]
default = ["mmap"]
```

Enable `fornax-tiff/mmap` from `fornax-raw-decoder`. The no-default-features
build uses `Arc<Vec<u8>>` directly from `fs::read`.

- [x] **Step 4: Implement source ownership**

Add `TiffSource`, `parse_source`, and `ParsedTiff::root()`. Change
`ParsedTiff.source` from `Arc<[u8]>` to `TiffSource`. Keep
`parse(&[u8])` for in-memory callers.

- [x] **Step 5: Add mapped opening at the decoder boundary**

In `fornax-raw-decoder`, when `mmap` is enabled, open the file read-only and map
it with `memmap2::Mmap`. Otherwise read to `Vec<u8>`. Call
`DngImage::parse_source`, not `DngImage::parse(&vec)`, so the file bytes are
not copied into a second allocation.

- [x] **Step 6: Update callers mechanically**

Replace `parsed.root` with `parsed.root()`. Remove the clone in:

```rust
let root = (*directories.first()?).clone();
```

Update DNG, Sony, Canon, Nikon, Hasselblad, Panasonic, and any other users of
`ParsedTiff`.

- [x] **Step 7: Verify feature matrices and mapped lifetime behavior**

Run:

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-raw-decoder --no-default-features
cargo test -p fornax-raw-decoder
cargo tree -p fornax-raw-decoder --no-default-features --edges normal
```

Also add a read-only mapped fixture test that decodes after the opening
function returns, proving the owner remains live.

- [x] **Step 8: Benchmark acceptance**

Run the 5D3 uncompressed and FP32 Deflate cases. Acceptance:

- uncompressed `fornax/end_to_end <= 1.5 * rawler/decode`;
- no regression in parse-only or end-to-end FP32 timing;
- source-copy and root-clone removal are visible in PR evidence.

- [x] **Step 9: Verify and commit**

Run `scripts/agent-validation.sh`.

```bash
git add Cargo.toml Cargo.lock crates
git commit -m "perf: make TIFF source storage non-copying"
```

**Expected gain:** This removes one full-file copy, repeated `fs::read`
allocation cost, and the duplicate root `BTreeMap`. The investigation measured
115 ms parser/setup time for a 48 MB uncompressed DNG.

**Risks and rollback:** Mapping can raise `SIGBUS` if another process truncates
the file; document that behavior and keep the owned-buffer feature fallback.
Disable the `mmap` default or revert this PR if platform behavior is unstable.
The `root()` migration is mechanical and can be separated into its own commit
inside the PR for easier review.

---

## Task 4: Borrowed DNG Metadata During Decode

**Files:**

- Modify: `crates/fornax-tiff/src/lib.rs`
- Modify: `crates/fornax-tiff/src/raster.rs`
- Modify: `crates/fornax-tiff/tests/raster.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/process_metadata.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/transforms.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/opcodes.rs`

**Interfaces:**

- Consumes: Task 3’s stable root access.
- Produces:

```rust
pub trait DirectoryLookup {
    fn get(&self, tag: &Tag) -> Option<&Value>;
    fn contains_key(&self, tag: &Tag) -> bool;
}

impl RasterLayout {
    pub fn from_lookup<D: DirectoryLookup + ?Sized>(
        directory: &D,
    ) -> Result<Self, RasterError>;
}

impl<'a> RasterRequest<'a> {
    pub fn new_with_lookup<D: DirectoryLookup + ?Sized>(
        source: &'a [u8],
        directory: &D,
        compression: DecodeCompression,
        bits_per_sample: u16,
        sample_format: SampleFormat,
        byte_order: ByteOrder,
        predictor: Predictor,
    ) -> Result<Self, RasterError>;
}
```

- [x] **Step 1: Add lookup precedence tests**

Create root `ImageWidth=10`, raw `ImageWidth=20`. Assert a
`DngDirectory` lookup returns 20, falls back to root for a root-only tag, and
matches `dng_metadata()` for every tag present in either directory.

- [x] **Step 2: Run the tests and verify they fail**

Run:

```bash
cargo test -p fornax-tiff directory_lookup
cargo test -p fornax-dng metadata_overlay
```

Expected: missing trait/overlay symbols.

- [x] **Step 3: Implement generic container lookup**

Add `DirectoryLookup` for `Directory`. Preserve existing methods:

- `RasterLayout::from_directory` delegates to `from_lookup`.
- `RasterRequest::new` delegates to `new_with_lookup`.

No DNG-specific tag names enter `fornax-tiff`.

- [x] **Step 4: Implement `DngDirectory<'a>`**

The crate-private overlay stores `root: &'a Directory` and
`raw: &'a Directory`. Raw wins on duplicate keys. Add an iterator only if a
decode helper still requires full iteration; do not materialize a merged map.

- [x] **Step 5: Route decode through the overlay**

Change `raw_mosaic`, `cropped_mosaic`, `process_metadata`, decode-plan helpers,
raster creation, transforms, and linearization/opcode lookups to use the
overlay. Keep `dng_metadata()` returning an owned merged `Directory` for
callers that explicitly need serialization/writing.

- [x] **Step 6: Prove full-DNG equivalence**

Compare old `dng_metadata()` plus general decode against overlay decode on every
selected DNG. Exact raw, cropped, process-metadata, and DNG-metadata hashes must
match the prior golden hashes.

- [x] **Step 7: Benchmark acceptance**

Run the Pixel 8 and 5D3 LJPEG cases. Acceptance:

- Pixel 8 removes about 100 ms of metadata cloning;
- no material regression on cases where the clone was below 1 ms;
- `fornax/end_to_end` improves without changing pixel bytes.

- [x] **Step 8: Verify and commit**

Run:

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-raw-decoder
scripts/agent-validation.sh
```

```bash
git add crates/fornax-tiff crates/fornax-dng
git commit -m "perf: borrow DNG metadata during decode"
```

**Expected gain:** About 103 ms on Pixel 8, roughly 8–10% of its original
end-to-end time, with additional reduction in peak memory.

**Risks and rollback:** A mistaken precedence rule can change geometry or color
metadata. The full overlay-vs-merged test and golden hashes are mandatory.
Revert this PR independently; the owned `dng_metadata()` API remains unchanged.

---

## Task 5: Compact Large Private TIFF Byte Payloads

**Files:**

- Modify: `crates/fornax-tiff/src/value.rs`
- Modify: `crates/fornax-tiff/src/parser.rs`
- Modify: `crates/fornax-tiff/src/writer.rs`
- Modify: `crates/fornax-tiff/tests/unknown_fields.rs`
- Modify: `crates/fornax-dng/src/metadata.rs`
- Modify: value consumers that need byte-list compatibility

**Interfaces:**

- Consumes: `Value` public conversion and serialization APIs.
- Produces:

```rust
pub enum Value {
    // existing variants...
    Bytes(Vec<u8>),
}

const LARGE_BYTE_VALUE_THRESHOLD: usize = 64;
```

- [ ] **Step 1: Add parser representation tests**

Create TIFF UNDEFINED fields with 64 and 65 bytes. Assert:

```rust
assert!(matches!(small, Value::List(_)));
assert!(matches!(large, Value::Bytes(_)));
```

Assert both expose identical byte lengths, indices, unsigned conversion, JSON,
and writer output.

- [ ] **Step 2: Run the tests and verify the variant is absent**

Run:

```bash
cargo test -p fornax-tiff large_byte
```

- [ ] **Step 3: Implement byte-array semantics**

Update `count`, scalar access, unsigned conversion, display, JSON conversion,
DNG helpers, and TIFF/DNG list writers. Preserve exact element order and byte
values. Small values continue using `List(Vec<Value::Byte>)`.

- [ ] **Step 4: Verify all byte consumers**

Audit opcode lists, DNG private data, MakerNotes, CFA patterns, and vendor
private blobs. Add helper methods rather than matching `Value::Bytes` in every
callsite:

```rust
impl Value {
    pub fn as_byte_slice(&self) -> Option<&[u8]>;
    pub fn into_byte_vec(self) -> Option<Vec<u8>>;
}
```

- [ ] **Step 5: Prove metadata and decode equality**

Run the Pixel 8 metadata JSON, golden hashes, and three-way raw equality. The
3.16-million-element private tag must serialize unchanged while avoiding one
`Value` allocation per byte.

- [ ] **Step 6: Benchmark acceptance**

Measure Pixel 8 parse/setup and end-to-end. Target:

- reduce parse/setup materially from the reported ~152 ms;
- no regression in any golden DNG output or writer benchmark.

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-raw-decoder
cargo bench -p fornax-dng --bench writer --no-run
scripts/agent-validation.sh
```

```bash
git add crates/fornax-tiff crates/fornax-dng
git commit -m "perf: store large TIFF byte values compactly"
```

**Expected gain:** Large reduction in Pixel 8 parsing allocation and peak
memory; estimated 8–15% end-to-end after the borrowed metadata change.

**Risks and rollback:** `Value` is shared by every format. Treat any conversion
or writer change as cross-format. If a consumer cannot preserve semantics,
revert this PR or raise the threshold temporarily without changing the public
decode path.

---

## Task 6: Direct Uncompressed Sample Unpacking

**Files:**

- Modify: `crates/fornax-codec/src/samples.rs`
- Modify: `crates/fornax-codec/src/uncompressed.rs`
- Modify: `crates/fornax-codec/tests/decode_contract.rs`

**Interfaces:**

- Consumes: `DecodeRequest` geometry and existing sample buffer output.
- Produces private:

```rust
fn unpack_row_u16(
    source: &[u8],
    target: &mut [u16],
    bits: u16,
    byte_order: ByteOrder,
);

fn unpack_12_msb(source: &[u8], target: &mut [u16]);
fn unpack_12_lsb(source: &[u8], target: &mut [u16]);
fn unpack_14_msb(source: &[u8], target: &mut [u16]);
fn unpack_14_lsb(source: &[u8], target: &mut [u16]);
```

- [ ] **Step 1: Add scalar-oracle tests**

Compare each grouped kernel with the current generic bit pump for randomized
rows, odd sample counts, both byte orders, and 12-, 14-, and 16-bit samples.
Assert every U16 value.

- [ ] **Step 2: Run the tests and verify missing symbols**

Run:

```bash
cargo test -p fornax-codec unpack_12
cargo test -p fornax-codec unpack_14
```

- [ ] **Step 3: Add a native-endian U16 copy**

When `bits == 16` and file endianness equals host endianness, copy source bytes
directly into the destination `u16` byte slice. Keep scalar/endian-swap paths
for other combinations.

- [ ] **Step 4: Add grouped 12/14-bit kernels**

Decode two 12-bit samples from three bytes and four 14-bit samples from seven
bytes, using word loads and scalar tails. Write directly to U16; delete the
`Vec<u32>` staging path for integer samples up to 16 bits.

- [ ] **Step 5: Preserve edge tiles**

Pass the exact copy width per row and test partial right-edge tiles for both
grouped formats. Do not read beyond the encoded payload.

- [ ] **Step 6: Prove full-corpus equality and benchmark**

Run three-way equality on all selected DNGs, especially Sigma fp 4K/2K and 5D3
uncompressed. Acceptance:

- Sigma fp 4K `fornax/end_to_end <= 1.5 * rawler/decode`;
- 5D3 uncompressed no slower than its Task 3 result;
- no output hash changes.

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo test -p fornax-codec
cargo test -p fornax-raw-decoder
scripts/agent-validation.sh
```

```bash
git add crates/fornax-codec
git commit -m "perf: unpack uncompressed DNG samples in place"
```

**Expected gain:** The investigation measured 43 ms of Fornax uncompressed
pixel work versus 14.9 ms for FlashRaw. The 12/14-bit grouped paths should
remove most of the scalar bit-pump and `Vec<u32>` overhead.

**Risks and rollback:** Partial trailing groups and opposite-endian payloads are
the main risks. Keep the previous generic functions as the scalar oracle and
fallback. Revert this PR independently.

---

## Task 7: Fused Deflate Predictor and Typed Conversion

**Files:**

- Modify: `crates/fornax-codec/src/deflate.rs`
- Modify: `crates/fornax-codec/src/predictor.rs`
- Modify: `crates/fornax-codec/src/samples.rs`
- Modify: `crates/fornax-codec/tests/decode_contract.rs`
- Modify: `crates/fornax-codec/README.md`

**Interfaces:**

- Consumes: typed destination buffers and current `Predictor` values.
- Produces private:

```rust
struct DeflateScratch {
    compressed: Vec<u8>,
    rows: Vec<u8>,
}

fn decode_float_rows_into(
    inflated: &[u8],
    width: usize,
    height: usize,
    bytes_per_sample: usize,
    predictor: Predictor,
    byte_order: ByteOrder,
    output: &mut [f32],
)
    -> Result<(), CodecError>;
```

- [ ] **Step 1: Add old-vs-fused equivalence tests**

For predictor 3, 34894, and 34895; 16/24/32-bit float; both byte orders; odd
widths; and multiple rows, compare the fused path bit-for-bit with the existing
`predictor::reverse` plus typed-conversion path.

Add integer horizontal predictor coverage if a synthetic request is available.

- [ ] **Step 2: Run the tests and verify the fused path is absent**

Run:

```bash
cargo test -p fornax-codec deflate_fused
```

- [ ] **Step 3: Reuse per-band scratch**

Allocate one `DeflateScratch` per parallel destination band or worker. Clear
and reuse vectors between tiles. Inflate into the reusable compressed/output
buffer and retain capacity.

- [ ] **Step 4: Fuse predictor and conversion**

For floating-point predictor tiles, reverse byte differencing, unshuffle byte
planes, convert f16/f24/f32/f64-era storage to F32, and write the typed row
directly. For native F32 storage with no predictor, decode directly without a
byte-plane pass. Keep the general path for unsupported combinations and error
reporting.

- [ ] **Step 5: Preserve Exif/byte-order behavior**

Test both endian variants and the current f16 expansion rules, including
subnormals, infinities, and signed zero.

- [ ] **Step 6: Prove full-corpus equality and benchmark**

Run three-way equality on FP16 and FP32 5D3 files. Acceptance:

- raw F32 bits remain exact;
- FP16 and FP32 absolute times decrease;
- Rawler advantage is retained with a visible end-to-end table.

Do not change `flate2` backend features in this task.

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo test -p fornax-codec
cargo test -p fornax-raw-decoder
scripts/agent-validation.sh
```

```bash
git add crates/fornax-codec
git commit -m "perf: fuse DNG deflate predictor reconstruction"
```

**Expected gain:** Remove one writable payload set and two whole-tile passes.
The investigation attributed roughly 50–65% of the FP32 gap to this pipeline;
the backend switch alone was only about 2%.

**Risks and rollback:** Floating-point bit exactness and scratch lifetime are
the main risks. Keep the old three-pass implementation as a test oracle and
fallback. Revert the PR independently.

---

## Files Expected to Change

- `TODO.md` and this plan.
- `crates/fornax-raw-decoder/{Cargo.toml,src,benches,tests}`.
- `crates/fornax-tiff/{Cargo.toml,src,tests}`.
- `crates/fornax-codec/{src,tests,README.md}`.
- `crates/fornax-dng/{src,tests}`.
- ParsedTiff callers in vendor decoder crates.
- `crates/decoder-tracker/golden/*`, benchmark workflow, and benchmark docs.
- `docs/reference/decoders/dng-lossless-jpeg.md`.

## Validation

Each PR runs its focused tests plus:

```bash
scripts/agent-validation.sh
```

Final performance integration also runs:

```bash
RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
cargo test -p fornax-raw-decoder --test decoder_equivalence -- \
  --ignored --nocapture

pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv

cargo bench -p fornax-raw-decoder --bench decoder --no-run
```

The final benchmark evidence must show every integer DNG at or below about
1.5x Rawler, with no individual-file regression hidden by an aggregate.

## Suggested PR Split

1. `test: add decoder performance guardrails`
2. `fix: decode packed-row Blackmagic lossless JPEG`
3. `perf: fuse common lossless JPEG decoding`
4. `perf: make TIFF source storage non-copying`
5. `perf: borrow DNG metadata during decode`
6. `perf: store large TIFF byte values compactly`
7. `perf: unpack uncompressed DNG samples in place`
8. `perf: fuse DNG deflate predictor reconstruction`

The plan/TODO branch may be handed off separately. Each implementation PR must
state its predecessor dependency, benchmark command, before/after numbers,
equality result, and rollback.

## Not Doing

- SIMD kernels: the report found no evidence they are required to reach
  Rawler, and FlashRaw’s benchmark adapter selected its scalar CPU backend.
- `target-cpu=native`: the experiment was neutral or worse.
- `flate2` zlib backend replacement: the measured gain was only about 2%.
- Changing the default Rayon thread count or benchmark to multi-threaded
  comparison.
- Lossy JPEG/DCT, JPEG XL, Zstandard, GPU decode, or new vendor formats.
- General-purpose metadata API redesign beyond the large byte representation.
- Removing the general LJPEG decoder.
- Production dependencies on Rawler, FlashRaw, or their codec crates.
- Committed host-dependent timing baselines.

## Decisions and Notes

- The headline performance comparison is Fornax `decode + raw_mosaic` versus
  Rawler/FlashRaw `decode`. Parse-only and pixel-only items remain diagnostics.
- Rawler and FlashRaw `raw_mosaic` timing items are intentionally omitted.
  Their decode calls already produce decoded samples; the adapter methods only
  clone those samples into Fornax's benchmark types. Raw-mosaic support remains
  a preflight and equality requirement.
- Rawler is the acceptance target; FlashRaw is an upper reference, not a
  requirement.
- The Blackmagic failure is a lossless row-packing bug, not a lossy DCT
  feature. Its SOF3 stream has the same sample count reshaped from 2064x1088 to
  1032x2176 per tile.
- `memmap2` is the only planned dependency. It reuses the already locked
  transitive version and is feature-gated with an owned-buffer fallback.
- No new CLI tool is needed. Pixi declarations remain unchanged.
- Implementation must stop after each independently mergeable PR until the user
  requests the next item.
