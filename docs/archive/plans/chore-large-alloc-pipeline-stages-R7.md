# R7: DNG Transform Allocations

## Result

The DNG OpcodeList1 and linearization full-size outputs now allocate through
`fornax-large-alloc`. The allocator-backed OpcodeList1 intermediate covers the
`f32` sample domain and converted `u8`, `u16`, and `u32` outputs. Linearization
uses the shared policy for its `u16` output.

The deinterleave output was tested through the same policy and reverted: on a
synthetic 16-megapixel input it regressed from 17.503 ms to 24.015 ms. It keeps
its existing seed-filled vector even though fault counts fall.

`fornax-dng` now has a default-on `large-page-hint` feature that forwards to
`fornax-large-alloc`; the direct allocator dependency has its own defaults off.

## Transform Comparison

Synthetic 4096x4096 `u16` images, with the large-page hint on/off. Three fresh
processes by five transform rounds. Values are median `[min-max]`.

| Transform | Metric | Hint off | Hint on | Change |
| --- | --- | ---: | ---: | ---: |
| Deinterleave | wall ms | 17.503 `[15.651-20.118]` | 24.015 `[20.421-29.378]` | Reverted |
| Deinterleave | minor faults | 102331 `[102229-102631]` | 62705 `[62570-62838]` | -38.7%, but slower |
| OpcodeList1 | wall ms | 169.520 `[166.645-174.385]` | 147.307 `[140.284-156.231]` | -13.1% |
| OpcodeList1 | minor faults | 212909 `[212847-213253]` | 87541 `[87491-87985]` | -58.9% |
| OpcodeList1 | peak RSS KiB | 167440 `[167296-167620]` | 167796 `[167532-167960]` | flat |
| Linearization | wall ms | 66.748 `[66.365-67.492]` | 38.563 `[38.106-52.773]` | -42.2% |
| Linearization | minor faults | 112171 `[111530-112253]` | 68089 `[67957-68426]` | -39.3% |
| Linearization | peak RSS KiB | 102048 `[101984-102048]` | 101932 `[101808-102060]` | flat |

The deinterleave measurements are the reason it is not routed through the
allocator. The temporary probe was removed before commit.

## Reference Equality

CLI TIFF output before and after:

| Input | SHA-256 |
| --- | --- |
| 5D3 uncompressed | `28915a249f3e79a2880a20333e424b057d044e3757697a3de190abd43d89053c` |
| Sigma fp 4K | `92bde2f4ef681ae5b5245f8de52329483435197ad88d42b1e36a4b302ba1b8af` |

## Tests

- Counting allocator coverage now includes `u8` as well as `u16`, `u32`, and
  `f32`, with matching natural-alignment allocation/drop layouts.
- Existing interleave, OpcodeList1, and linearization value tests remain green.
- A policy assertion checks the two kept transform outputs call the shared
  allocator instead of collecting unaccounted full-size vectors.

## Validation

Passed:

- `cargo test --workspace --all-features --exclude compare-demosaic`
  (681 passed, 3 ignored)
- `cargo test -p fornax-dng --no-default-features`
- `cargo test -p fornax-large-alloc --no-default-features`
- `cargo check -p fornax-dng -p fornax-large-alloc` for Windows MSVC and macOS
  arm64, with `large-page-hint` on and off
- `git diff --check`

Miri is not installed for nightly. Full all-features remains blocked by the
uninitialized `compare-demosaic` reference sources.

## Unsafe

This item adds no unsafe code. Production unsafe remains limited to the two
existing allocator blocks.
