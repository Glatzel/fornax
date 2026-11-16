# R6a: Independent Large-Allocator Crate

## Result

The allocator moved from `fornax-codec` to the independent base-workspace crate
`fornax-large-alloc` with a history-preserving `git mv`. This item is a
mechanical extraction: allocation policy, threshold, feature default, and
decoded bytes are unchanged.

`fornax-large-alloc` owns `AllocError`, defaults `large-page-hint` on, keeps
`libc` Linux-only and optional, and is marked `publish = false`. The
`fornax-codec` feature forwards to the crate and maps `AllocError` to
`CodecError` in one local adapter.

## Name Check

Checked before creating the crate:

| Namespace | Result |
| --- | --- |
| Workspace manifests and `Cargo.lock` | No `fornax-large-alloc` match |
| crates.io search | No package match |

## Safety Surface

The new crate root has `#![deny(unsafe_code)]`; only
`crates/fornax-large-alloc/src/large_alloc.rs` has
`#[allow(unsafe_code)]`. `fornax-codec` now has its own crate-root
`#![deny(unsafe_code)]`.

Production contains exactly two unsafe blocks:

1. `alloc_zeroed` plus `Vec::from_raw_parts` adoption.
2. The `MADV_HUGEPAGE` call.

The counting-allocator test adds one `unsafe impl GlobalAlloc` with three
unsafe trait methods and three unsafe delegation calls. These are test-only and
remain in the same allocator module.

## Decode Smoke Comparison

This is a no-behavior-change sanity check, not a performance claim. Three fresh
processes by five decode rounds at one Rayon worker. Values are median
`[min-max]` over 15 fresh-process decode timings.

| Input | Metric | Plan branch | New crate | Change |
| --- | --- | ---: | ---: | ---: |
| 5D3 uncompressed | decode ms | 17.714 `[16.938-39.882]` | 17.507 `[16.998-18.826]` | -1.2% |
| 5D3 uncompressed | minor faults | 12272 `[12260-12999]` | 12233 `[12181-12284]` | -0.3% |
| 5D3 uncompressed | peak RSS KiB | 97844 `[97840-97980]` | 97928 `[97820-97980]` | +0.1% |
| 5D3 FP32 Deflate | decode ms | 352.303 `[348.184-383.983]` | 356.278 `[353.644-377.044]` | +1.1% |
| 5D3 FP32 Deflate | minor faults | 79588 `[77756-80820]` | 79407 `[79213-79901]` | -0.2% |
| 5D3 FP32 Deflate | peak RSS KiB | 151640 `[151396-151692]` | 151636 `[151536-151708]` | flat |

The FP32 difference is about one percent with overlapping process noise and no
new work on the measured path; both variants remain in the same session.

## Tests

Passed:

- `cargo test -p fornax-large-alloc`
- `cargo test -p fornax-large-alloc --no-default-features`
- `cargo test -p fornax-codec --all-features` (108 passed, 2 ignored)
- `cargo test -p fornax-codec --no-default-features` (60 passed)
- `cargo test --workspace --all-features --exclude compare-demosaic`
  (679 passed, 3 ignored)
- `cargo check -p fornax-large-alloc` for Windows MSVC and macOS arm64, with
  `large-page-hint` on and off
- `cargo check -p fornax-codec --no-default-features` for Windows MSVC and
  macOS arm64
- `git diff --check`

The decoder three-way equality gate passed:

```text
PASS,fornax,5920x3950,U16,bits=16,sample_format=1
PASS,rawler,5920x3950,U16,bits=16,sample_format=1
PASS,flashraw,5920x3950,U16,bits=16,sample_format=1
```

Caveats:

- `cargo check -p fornax-codec --all-features` cannot cross-compile because
  `zstd-sys` needs an MSVC or macOS C toolchain unavailable on this host. The
  feature-off checks pass.
- Miri is not installed for nightly.
- The pure allocator tests cover zero/one/below/at/above threshold, natural
  alignment, matching drop layout, zero filling, capacity, and the forced
  portable path. The real-kernel `AnonHugePages` evidence remains recorded in
  the existing DNG profile report; no flaky `/proc/self/smaps` assertion was
  added to unit CI.
