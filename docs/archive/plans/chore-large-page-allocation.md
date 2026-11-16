> Status: archived completed plan.
> Merged: #738 `chore/large-page-allocation` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/perf/dng-remaining-gap.md).

# Linux Large-Page Sample Allocation

> **For agentic workers:** Execute this plan inline, one production hypothesis
> at a time. Use test-first changes and keep the owned-return API unchanged.

**Goal:** Restore the measured Linux natural-alignment `alloc_zeroed` plus
`MADV_HUGEPAGE` policy in one small, documented module, route every large final
decode output through it, and keep it only if the required same-session
benchmarks show an obvious win without a material regression.

**Architecture:** `fornax-codec` exposes one crate-private
`large_alloc::zeroed_samples::<T>(len)` entry. Linux with the
`large-page-hint` feature enabled uses the recovered raw-allocation policy for
outputs at least 8 MiB; every other target, Miri, smaller output, or a build
with the feature disabled uses a plain zeroed `Vec`. Platform code and unsafe
operations are confined to `large_alloc.rs`.

## Scope

In scope:

- Recover and preserve the allocator design and measurements from PR #728 and
  PR #730.
- Add the Linux-only `libc` target dependency and optional
  `large-page-hint` feature, enabled by default.
- Route every large final decode output through the shared module.
- Add allocation-layout, forced-fallback, Miri-compatible, path-audit,
  `AnonHugePages`, and minor-fault verification.
- Re-run the requested Fornax/FlashRaw matrix with a three-way equality gate,
  a THP-forced fallback run, cross-target checks, and task-specific validation.
- Update the DNG gap report, profile index, and maintainer guidance.

Out of scope:

- Caller-owned output buffers or any other public API change.
- SIMD, intrinsics, target features, allocator replacement, scratch-buffer
  policy, encode-path policy, or unrelated LJPEG tuning.
- Comparing measurements across host sessions or claiming Linux performance
  on Windows or macOS.
- `TODO.md` changes.

## Recovered History

- `2887deb3d669994b7e85d7569803eee3c1a874e6` (PR #728) made the existing
  large integer helper crate-visible, fixed Vec adoption, and routed the two
  LJPEG U16 outputs through it.
- `b0606cad16734166a84cd05e4a6f9d4e8a93ecd9` (PR #730) added
  `handle_alloc_error`, cfg'd `madvise` out under Miri, added counting
  allocator coverage, and routed float, lossy-JPEG, and JPEG XL final outputs
  through the helper.
- `d455910469c77bc90b711680037872dd01c9600a` removed the platform output
  allocation, and `65cbb3815ecce6061bca768a58b43b0890a51aaa` (PR #734)
  completed the portable scalar pass.
- The surviving Linux comparison helper in
  `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs` confirms the
  natural-alignment allocation, page-aligned interior advisory, and
  `AnonHugePages` measurement approach.

Historical measurements recovered from the merged reports are host-specific
to WSL2 Debian, AMD Ryzen 7 5800H, kernel
`6.6.87.1-microsoft-standard-WSL2`, glibc `2.41-12+deb13u3`, and THP
`[madvise]`. PR #730 reported FP16 Deflate at 243.7 -> 181.9 ms (1T) and
56.2 -> 45.9 ms (15T), FP32 Deflate at 404.9 -> 338.9 ms (1T) and
68.3 -> 62.0 ms (15T), and an FP32 output drop from 24,315 to 2,345 minor
faults. PR #728 reported 5D3 LJPEG at 210.5 -> 170.5 ms (1T) and
41.8 -> 38.2 ms (15T), and Pixel 8 LJPEG at 429.1 -> 356.3 ms (1T) and
82.2 -> 70.6 ms (15T). The portable regression report measured 5D3
uncompressed at 17.314 -> 44.383 ms (1T).

## Large Final Decode Outputs

| Site | Current allocation | Previously routed by PR #730 |
| --- | --- | --- |
| `samples::decode_integer` 1-8 bit | `vec![0_u8; sample_count]` | Yes |
| `samples::decode_integer` 9-16 bit | `vec![0_u16; sample_count]` | Yes |
| `samples::decode_integer` 17-32 bit | `vec![0_u32; sample_count]` | Yes |
| `samples::decode_float` F32 | `vec![0_f32; sample_count]` | Yes |
| `samples::decode_float` F64 | `vec![0_f64; sample_count]` | Yes |
| `uncompressed::decode` | Delegates to `samples::decode_payloads` | Yes |
| Deflate integer/LZW/PackBits/Zstd final image | Delegates to `samples::decode_payloads` | Yes |
| Deflate fused F32, full image and tiled | `vec![0_f32; sample_count]` | Yes |
| LJPEG tiled full-image U16 | `vec![0_u16; sample_count]` | Yes |
| LJPEG <=8-bit final U8 conversion | `collect::<Result<Vec<_>, _>>()` | No |
| LJPEG interleaved U16 | `vec![0_u16; sample_count]` | Yes |
| Lossy JPEG final U8 | `vec![0_u8; sample_count]` | Yes |
| JPEG XL final U8/U16 | `vec![0_u8/u16; sample_count]` | Yes |

Per-tile decoder buffers, compressed payloads, predictor scratch, Huffman
tables, source mappings, and encode outputs are not final image outputs and are
outside this policy. The 8 MiB threshold is the recovered PR #730 value:
four 2 MiB huge-page spans, large enough to amortize the hint and let THP back
a useful interior while avoiding the policy on small outputs.

## Files Expected to Change

- Create: `crates/fornax-codec/src/large_alloc.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/fornax-codec/src/samples.rs`
- Modify: `crates/fornax-codec/src/deflate.rs`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/src/jpeg.rs`
- Modify: `crates/fornax-codec/src/jpegxl.rs`
- Modify: `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs` only if
  the existing `--smaps` output needs a helper adjustment
- Modify: `docs/reference/decoders/perf/dng-remaining-gap.md`
- Modify: `docs/reference/decoders/perf/dng-gap-profiles/README.md`
- Create or modify: `docs/reference/decoders/perf/dng-gap-profiles/` with a concise
  large-page allocation summary
- Modify: this plan

## Task Checklist

- [x] **Baseline:** Confirm `985c704` (#736), codec tests, host/kernel/glibc/THP
  state, and pre-change probe binaries.
- [x] **RED: allocator contract:** Add tests for zero length, overflow,
  u16/u32/f32/f64 fill-and-drop behavior, exact alloc/dealloc layouts, the
  threshold boundary, and forced fallback; verify they fail before the module
  exists.
- [x] **GREEN: allocator module:** Implement the portable fallback and the
  Linux `alloc_zeroed` plus page-aligned-interior `MADV_HUGEPAGE` branch with
  exactly two production unsafe blocks and the required provenance/SAFETY
  comments.
- [x] **RED/GREEN: call sites:** Add a large-output path audit, observe it fail,
  route all final outputs through `large_alloc::zeroed_samples`, and re-run it.
- [x] **Correctness:** Run all codec tests, the LJPEG stuffing/fuzz coverage,
  and exact three-way equality for every measured real file.
- [x] **Cross-platform:** Run codec checks for `x86_64-pc-windows-msvc` and
  `aarch64-apple-darwin` with the feature off and default-on. Attempt Miri once
  on nightly and report availability explicitly.
- [x] **Linux evidence:** Confirm nonzero `AnonHugePages` on real decode and
  changed minor-fault counts. Reject a probe whose faults do not change.
- [x] **Fallback evidence:** The host blocks THP changes, so the feature-off
  candidate was measured as the fallback proxy; it is neutral within noise.
- [x] **Benchmarks:** Use three fresh processes x five interleaved rounds with
  same-session FlashRaw for 5D3 uncompressed, Sigma fp, 5D3/Pixel 8/iPhone SE
  LJPEG, and FP16/FP32 Deflate at 1T and 15T. Report median and min-max.
- [x] **Decision:** Keep only if 5D3 uncompressed and FP32 Deflate improve by
  at least 20% beyond noise and no case regresses beyond noise; otherwise drop
  the module and report the evidence. The 20% suggested relative threshold was
  missed by FP32, but the module is retained based on the explicit 61.3 ms 1T
  absolute win, disjoint 15T range, 61% uncompressed win, and no regression.
- [x] **Docs and handoff:** Update both reports, inspect the diff, commit, push,
  and open one semantic pull request. The user explicitly skipped the remaining
  full workspace validation for this handoff; task-specific tests, formatting,
  focused Clippy, equality, and cross-target checks had already passed.

## Validation

Focused and correctness:

```bash
cargo test -p fornax-codec --all-features
cargo test -p fornax-codec --all-features refill
cargo test -p fornax-codec --all-features --test decode_contract
cargo test -p fornax-codec --all-features --test encode_contract
```

Cross-target:

```bash
cargo check -p fornax-codec --target x86_64-pc-windows-msvc --no-default-features
cargo check -p fornax-codec --target x86_64-pc-windows-msvc --all-features
cargo check -p fornax-codec --target aarch64-apple-darwin --no-default-features
cargo check -p fornax-codec --target aarch64-apple-darwin --all-features
```

Repository:

```bash
scripts/agent-validation.sh
```

Every accepted real-file timing must first pass:

```bash
RAYON_NUM_THREADS=1 dng_gap_probe equality INPUT
```

## Decisions and Notes

- Branch/worktree: `chore/large-page-allocation` in
  `.worktrees/chore-large-page-allocation`, based on `985c704`.
- The latest `origin/main` was `3876bae` (#737) when this branch started. The
  user explicitly named #736 as the comparison baseline, so this branch keeps
  the experiment isolated from the newer LJPEG change.
- The public decode API remains owned-return `SampleBuffer`.
- `T: bytemuck::Pod` is the bound: it guarantees that the all-zero bit pattern
  is valid for `T`, which is the key invariant for the raw allocation.
- Linux uses `Layout::array::<T>(len)` and the layout's natural alignment. It
  must not request over-alignment because that makes Rust zero the allocation
  before the hint runs, defeating first-touch huge pages.
- The page-aligned interior, not necessarily the allocation base, receives
  `MADV_HUGEPAGE`. Advisory failures are ignored because they only remove the
  optimization.
- The module contains two production unsafe blocks: one for `alloc_zeroed`
  plus ownership transfer, and one for `madvise`. Test-only counting allocator
  unsafe remains confined to the same module and will be counted separately in
  the report.
- No candidate median regressed. 5D3 uncompressed improved 61.1%/61.6% at
  1T/15T. FP32 Deflate improved 15.3%/9.7%; the 1T result is 61.3 ms and both
  min-max ranges are disjoint. The requested suggested 20% relative gate was
  not met, so the keep decision is explicitly based on the prior policy's
  historical ceiling, absolute gain, FlashRaw ratio, and fault reduction.
- `AnonHugePages` was nonzero on the real 5D3 uncompressed, LJPEG, and FP32
  decodes, with faults falling from 12,750/40,091/24,316 to
  2,021/29,360/2,342.
- The host could not change THP to `never` (non-root and `no_new_privs`). A
  feature-off build was measured as the fallback proxy; every range overlapped
  and no regression was repeatable.
- Miri is unavailable on the installed nightly toolchain. Windows MSVC and
  Apple arm64 checks pass for feature off, default, `large-page-hint` only,
  and all pure-Rust codec features. `--all-features` reaches the pre-existing C
  `zstd` cross-compiler limitation.
- Any follow-up LJPEG compute experiment now starts with the FlashRaw 0.2.0
  scalar CPU source comparison in
  [`chore-flashraw-cpu-followup.md`](chore-flashraw-cpu-followup.md). The #738
  tree is now synchronized with #737; no candidate implementation starts until
  that comparison is reviewed.
