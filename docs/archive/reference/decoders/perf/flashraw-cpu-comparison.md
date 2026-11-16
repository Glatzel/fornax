> Status: archived historical source comparison.
> Current document: [FlashRaw versus Fornax raw decode](../../../../reference/decoders/perf/flashraw-vs-fornax.md).
> Archived 2026-11-16; version-specific implementation details may be stale.

# FlashRaw 0.2.0 CPU Source Comparison

Source-only comparison for the PR #738 allocator baseline after its merge with
`main`. FlashRaw 0.2.0 is MIT OR Apache-2.0. This note records design
observations only; no FlashRaw code was copied. If a later implementation
borrows an idea, its source comment must name the FlashRaw-version path and line
and the MIT OR Apache-2.0 licence.

FlashRaw sources were read from the pinned crates.io checkout used by
`flashraw = "=0.2.0"`:

- `flashraw-codecs-0.2.0/src/ljpeg.rs`
- `flashraw-codecs-0.2.0/src/ljpeg_par.rs`
- `flashraw-core-0.2.0/src/bits.rs`
- `flashraw-core-0.2.0/src/huffman.rs`
- `flashraw-core-0.2.0/src/image.rs`
- `flashraw-formats-0.2.0/src/dng.rs`

Only the scalar `Backend::Cpu` design was considered. SIMD and GPU paths were
ignored.

## Side by Side

| Area | FlashRaw 0.2.0 CPU | Fornax at PR #738 | Consequence |
| --- | --- | --- | --- |
| Huffman table and entry | One 12-bit, 4,096-entry `u32` lookup per table (`src/huffman.rs:38`, `:62`). Entry bits carry consumed length, full/valid flags, symbol, and optionally the already decoded difference (`:41`). A canonical max-code/value table handles long codes. | The synchronized #738 tree includes #737's 10-bit two-byte fast table plus compact canonical counts and symbols (`crates/fornax-codec/src/ljpeg.rs:21`, `:232`, `:245`). A 17-symbol table is about 2.1 KiB. | Fornax has a much smaller hot table; FlashRaw can precompute the full difference in more cases because it spends four times as many bytes per lookup. The remaining question is whether selected 12-bit full entries beat the current table's arithmetic. |
| Difference/sign extraction | Fast full entries return a precomputed difference. Non-full hits extract code and magnitude together and use branchless sign extension (`src/huffman.rs:17`, `:246`). | The fast path extracts code and magnitude together and uses branchless sign extension (`crates/fornax-codec/src/ljpeg.rs:347`, `:1289`). | The sign logic already matches the important FlashRaw idea. Only the trade between precomputed differences and table footprint remains open. |
| Bit-pump state and refill | State is a local `BitPumpMsb` value with data, position, 64-bit cache, valid-bit count, and overrun count (`src/bits.rs:114`). A normal refill consumes up to eight bytes at once (`:182`). Multi-component decoding batches refills so one refill usually covers two symbols (`src/ljpeg.rs:531`). | State is a local concrete `FastBitReader` with the same cache/position shape, an error slot, an eight-byte fast refill, and a byte-wise marker-aware slow path (`crates/fornax-codec/src/ljpeg.rs:476`, `:497`). The predictor-1 row loop calls `refill` once before every component (`:1158`). | FlashRaw performs fewer refill checks for two-component rows. This is a small, isolated hypothesis for PR #738. |
| Marker and stuffing handling | Removes `FF 00` stuffing once into a scratch scan before decoding, records restart segments, and can parallelize the unstuff pass for scans at least 2 MiB (`src/ljpeg.rs:333`, `:383`). The hot bit pump then sees plain bytes. | Handles stuffing and terminating markers inside `FastBitReader`: an eight-byte no-`FF` shortcut, otherwise a byte-wise slow path (`crates/fornax-codec/src/ljpeg.rs:497`). No full-scan copy. | FlashRaw trades one read/copy pass for a simpler refill loop. Prior Fornax unstuff experiments helped smooth streams but regressed noisy data, so this remains a conditional hypothesis rather than an assumed win. |
| Row loop and predictor storage | Predictor-1 rows keep one predictor per component in a small local array. Component counts 1-4 dispatch to fixed-size row loops; two- and four-component loops keep tables and predictors in register-friendly arrays (`src/ljpeg.rs:492`, `:531`). | Predictor-1 supports one or two components. Current and previous row scratch are reused per band; active predictions live in a two-element array (`crates/fornax-codec/src/ljpeg.rs:57`, `:1158`). | The general logic is already close. Specializing the component loop was previously measured and reverted, so it is not a top candidate without new counter evidence. |
| Output buffer and direct rows | DNG allocates the final `RawImage` first (`src/dng.rs:798`; `src/image.rs:213`). Tile writers hand a destination row to the LJPEG sink, which writes decoded rows directly into that output (`src/dng.rs:1122`; `src/ljpeg.rs:665`). A parallel entropy path may use a temporary difference frame, but the ordinary tiled CPU path is output-direct. | PR #738 allocates the final owned `Vec` first. The predictor-1 tile path decodes rows into reusable row scratch and writes each row directly into the destination band (`crates/fornax-codec/src/ljpeg.rs:47`, `:57`, `:81`). A non-fast fallback reuses a per-band tile buffer and then copies rows. | The common DNG path is already output-direct in both implementations. A public decode-into/sink API is not justified by this comparison and would require an API proposal. |
| Per-tile setup and reuse | Parses each tile independently and builds its Huffman tables and unstuff buffer per tile. DNG allocates one final image and claims tile regions from it (`src/dng.rs:849`). | Reuses row scratch and the fallback decoded buffer across tile columns in a band, but parses Huffman tables per tile (`crates/fornax-codec/src/ljpeg.rs:52`). | Fornax already reuses the obvious mutable scratch. Remaining per-tile Huffman setup is a known fault source, but prior table-reuse experiments did not improve wall time. |
| Large output allocation | Natural-alignment `alloc_zeroed`, 8 MiB threshold, Linux `MADV_HUGEPAGE` on the 2 MiB-aligned interior, otherwise a normal zeroed `Vec` (`src/image.rs:59`, `:81`, `:104`). | PR #738 implements the same policy in `large_alloc.rs`: 8 MiB threshold, natural allocation layout, page-aligned interior, Linux-only `libc`, default-on `large-page-hint`, and Miri/fallback exclusion (`crates/fornax-codec/src/large_alloc.rs:50`, `:52`, `:84`, `:109`). | No allocator hypothesis remains: the approaches agree. Keep platform code confined to `large_alloc.rs`. |
| Uncompressed DNG rows | Parses strips/tiles into a final `RawImage`, then unpacks each source row directly into its destination row (`src/dng.rs:912`, `:1045`). Scalar 16-bit byte order is a row copy-like loop. | Allocates the final image, then `copy_samples`/`unpack_row_u16` writes each row directly into the typed output (`crates/fornax-codec/src/samples.rs:97`, `:380`, `:535`). | No material scalar structural difference was found beyond allocation, which PR #738 already addresses. |

## Ranked Testable Hypotheses

These are proposals for measurement, not accepted changes.

1. **Test a fixed 12-bit packed-u32 full-difference table before any adaptive
   variant.**
   Testable claim: 4,096 packed `u32` entries (16 KiB) can precompute more
   complete differences than the post-#737 10-bit table without the old
   65,536-entry fallback. Keep the compact canonical counts/symbols slow path.
   Compare directly with the 10-bit table using cycles/sample and wall time;
   report instructions/sample and L1D misses/sample. Expected: 2-5% LJPEG wall
   improvement if large-difference rejection dominates; neutral or slower if
   the 16 KiB working set raises L1D pressure.

2. **Batch bit-reader refill across adjacent components.**
   Testable claim: in two-component predictor-1 scans, one refill can cover two
   decoded differences just as FlashRaw's paired refill sequence intends.
   Change the refill boundary without changing one-component scans. Judge by
   cycles/sample and wall time, reporting instructions and L1D misses.
   Expected: 1-3% fewer cycles/sample and 1-2% wall time on real
   two-component LJPEG; no measurable change on one-component scans.

3. **Parallelize predictor-1 entropy chains within one large scan only after a
   written boundary design.**
   Testable claim: a speculative difference decoder plus prefix reconstruction
   can improve all-thread decoding when a DNG has one large scan rather than
   many tiles. The design must explain how safe boundaries are found without
   restart markers, including phase/table synchronization, predictor chaining,
   disagreement detection, exact fallback, deterministic ordering, and error
   semantics. Expected: potentially 1.5-2x at 15T on the Pentax K-5 strip, no
   1T gain, and little or no gain on tiled 5D3/Pixel cases.

4. **Deferred: pre-unstuff each tile into reusable per-worker scratch.**
   Testable claim: removing marker checks from every refill beats the extra
   scan copy, especially when scratch is reused across tiles. Expected:
   approximately 1-4% on smooth one-thread scans, with possible regressions on
   noisy scans from added memory traffic. Prior evidence was mixed, so this is
   not currently scheduled.

Before any candidate, record the post-#737/#738 baseline for 5D3, Pixel 8,
iPhone SE, and the Pentax K-5 strip. Every row must include same-session
FlashRaw, cycles/sample, instructions/sample, L1D load misses/sample, and wall
median/min-max.

## Public API Boundary

FlashRaw exposes internal sink/caller-owned output entry points. Fornax's
current external contract returns owned `SampleBuffer` values. Adding a public
trait or caller-owned decode-into API solely to imitate FlashRaw is a separate
API proposal and is not part of these hypotheses. No public API change should
be made without explicit approval.

## Baseline Caveat

The post-#737/#738 baseline is merged `origin/main` commit `eaf7ec6`. Pentax
K-5 is now materialized and passes the strict Fornax/Rawler/FlashRaw equality
gate. Its 15T wall time is effectively the same as 1T, confirming the
restart-free single-scan parallelism opportunity.

The fixed 12-bit packed-`u32` table was tested first and reverted. It reduced
instructions and helped the tiled LJPEG files, but Pentax regressed by 16-17%
with higher cycles and L1D pressure. Component-pair refill was tested next and
also reverted: it increased real LJPEG instructions and wall time. The compact
10-bit table and current refill policy remain.

Single-scan parallelism is stopped at the written design gate in
[`restart-free-single-scan-parallelism-design.md`](../../../../reference/decoders/design/restart-free-single-scan-parallelism-design.md).
No parallel implementation was attempted. The complete baseline, ranges,
same-session FlashRaw comparison, and rejected-candidate counters are in
[`flashraw-cpu-followup.txt`](dng-gap-profiles/flashraw-cpu-followup.txt).
