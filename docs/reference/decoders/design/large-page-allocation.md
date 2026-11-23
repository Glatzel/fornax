# Linux large-page output allocation

## Decision

Fornax keeps its owned `Vec<T>` decode outputs while allowing Linux builds to
obtain first-touch and transparent-huge-page benefits through the
`fornax-large-alloc` crate. The decode API does not expose caller-owned output
buffers.

Linux builds with the default-on `large-page-hint` feature use:

- a natural-alignment `alloc_zeroed(Layout::array::<T>(len))` allocation;
- this path only for final outputs of at least 8 MiB;
- `MADV_HUGEPAGE` on the 2 MiB-aligned live interior;
- adoption of the allocation into the returned `Vec<T>` with
  `capacity == len`.

Windows, macOS, Miri, feature-off builds, and smaller outputs use a plain
zeroed `Vec`.

## Why

The 5D3 uncompressed DNG was the clearest case. The policy reduced single-thread
decode from 47.357 ms to 18.400 ms and brought Fornax to FlashRaw parity in the
same session. It also removed most first-touch faults, for example from 12,750
to 2,021 on that file. LJPEG and floating-point Deflate outputs improved by
smaller but repeatable amounts.

The public API and decoder behavior remain unchanged. Output allocation is not
an excuse to add a `decode_into` contract without a separate API decision.

## Invariants

- `Layout::array::<T>(len)` must describe exactly the returned Vec length and
  natural alignment. Do not introduce over-alignment: it forces zero-fill before
  `madvise` and disables the first-touch benefit.
- Allocation and `Vec::from_raw_parts` must use the same global allocator.
- The block must have `capacity == len`, a matching `T: Pod` drop layout, and
  `len == 0` handled without constructing an invalid allocation.
- Keep the 8 MiB threshold and 2 MiB page-aligned-interior calculation together.
- Re-run `AnonHugePages` and process minor-fault counters after Rust, glibc, or
  kernel changes. A timing without a fault or huge-page change is not evidence
  that the policy engaged.
- All platform code and unsafe allocation ownership stay in
  `crates/fornax-large-alloc`.

## Rejected alternatives

- `Vec::with_capacity` followed by `madvise` performed a second zero-fill and
  used more instructions than the natural `alloc_zeroed` path.
- Caller-owned LJPEG output reuse saved only about 1.6-5.4% in synthetic scans
  and 1.6% on a full 5D3 output, so it did not justify a new public API and
  partial-write semantics.
- Feature-off, native-CPU, LTO, reusable-unstuffing, and alternative Huffman
  table experiments were neutral, mixed, or regressions. Their detailed
  evidence remains in the archived DNG performance pack.

## Evidence

The reproducible probe is
`crates/fornax-raw/benches/dng_gap_probe`; the implementation is
`crates/fornax-large-alloc`. Detailed historical measurements are under
[the archived decoder performance evidence](../../../archive/reference/decoders/perf/).
