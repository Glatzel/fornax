# Golden decoder performance implementation plan

## Goal

Close four measured Fornax/FlashRaw 0.3.1 gaps in one branch while preserving
the accepted raw mosaic for every golden sample: packed DNG rows, the 14-bit
DNG kernel, four-component Sony LJPEG rows, and the Canon CRX Rice reader.

## Scope

In scope:

- Direct 8-bit, 10-bit, and 14-bit uncompressed DNG row unpacking into the
  final sample buffer, with row parallelism for single-strip images.
- A separate 14-bit packed-row kernel with no generic `u32` intermediate.
- A four-component Sony LJPEG row path that writes both Bayer rows without a
  complete JPEG result buffer, a second tile buffer, or a reshape pass.
- A Canon CRX Rice zero-prefix reader backed by a refilled 64-bit cache and
  `leading_zeros`.
- Focused tests, all required workspace checks, golden hashes, and the
  documented three-process benchmark after every step.
- Final performance-report and dated-results updates.

Out of scope:

- Nikon Huffman or D6 packed rows, DNG Deflate, Sony edge-completion memory
  behavior, Canon DNG LJPEG, single-scan LJPEG parallelism, and large-page
  allocation policy changes.
- Public facade or trait changes, new production dependencies, SIMD, target
  features, or platform-specific unsafe code.

## Task checklist

- [x] **Step 1: direct packed DNG rows**
  - Add failing contract tests for 8/10/14-bit MSB ordering with row widths
    whose packed ends are not byte-aligned.
  - Route 8-bit and 10-bit rows directly into typed output; keep 12-bit on its
    current path. Validate every strip before row-parallel decoding.
  - Benchmark all uncompressed DNG rows at 1T/16T. Commit only if the Sigma
    8/10-bit targets win without a beyond-spread regression.
- [x] **Step 2: 14-bit kernel**
  - Add a kernel test with hand-built 14-bit groups and tails.
  - Specialize the 14-bit four-sample group kernel separately from generic
    U16 rows. Keep one shared row dispatcher rather than duplicate the path.
  - Benchmark the G1 X row and all DNG rows; stop if 1T/16T are not wins.
- [x] **Step 3: four-component Sony LJPEG**
  - Add a 512-wide two-Bayer-row synthetic test.
  - Expose only the doc-hidden cross-crate LJPEG row decoder needed by Sony;
    decoder predictions stay in a fixed register array. The Sony sink writes
    the transformed rows directly into the final mosaic.
  - Fall back to the existing generic path for any scan that is not exactly
    four components. Benchmark Sony LJPEG and both Canon DNG LJPEG rows.
- [x] **Step 4: Canon CRX Rice**
  - Add prefix tests for zero length, the escape boundary and beyond, a
    refill boundary, and exhausted/corrupt input.
  - Replace the per-bit unary loop with a local 64-bit cached reader that uses
    `leading_zeros`, preserving the current consumption rule after the escape
    limit.
  - Benchmark Canon CRX; inspect all other rows for regressions.
- [x] Run final full validation, refresh the report and dated CSV, inspect all
  four commits, and leave the branch unpushed.

## Files expected to change

- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-codec/tests/decode_contract.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-raw-sony/src/codec/ljpeg.rs`
- `crates/fornax-raw-sony/src/codec/mod.rs`
- `crates/fornax-raw-canon/src/codec/mod.rs`
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`
- `docs/reference/decoders/perf/data/golden-flashraw-vs-fornax-2026-11-17.csv`
- `docs/plans/chore-decoder-golden-perf.md`

## Validation

After each step:

```bash
cargo test -p fornax-codec --features lossless-jpeg
cargo test -p fornax-raw-sony -p fornax-raw-canon
cargo run --release -p decoder-tracker -- hash --check \
  crates/decoder-tracker/golden/decode-hashes.csv
```

Run the probe with three independent processes per decoder/thread count, two
warmups and five rounds per process, warm page cache, bench profile, and
FlashRaw CPU backend. Compare targeted rows and format families that share the
edited path.

Final validation:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --check
scripts/agent-validation.sh
```

## Decisions and notes

- The checked-in tree already routes 14-bit rows away from the generic `u32`
  scratch. Step 1 therefore brings 8/10-bit rows to that direct-row design and
  step 2 owns the remaining 14-bit kernel cost; this avoids duplicating the
  scratch-removal change.
- The generic LJPEG row hook is `#[doc(hidden)]` internal plumbing, not a
  user-facing facade change. Only Sony invokes it, and only after identifying
  a four-component scan.
- Benchmark acceptance uses per-row medians and batch spread. A slower row is
  reported even when it is within spread; a regression beyond spread rejects
  the step.
- G1 X remains dominated by `OpcodeList1` conversion after the packed kernel.
  That work stayed out of scope and is now the next DNG-specific priority.
- The final unrelated `fornax-raw-pipeline` workspace-clippy warning remains
  pre-existing; changed packages are clean under strict clippy.
