# R6b: Shared Crop-And-Cast Helper

## Result

The seven decoder adapters now delegate cropping and sample conversion to one
helper in `fornax-raw-core`:

- DNG
- Canon
- Sony
- Hasselblad
- Nikon
- Panasonic
- Fujifilm

The helper crops in place, converts in parallel into a typed vector allocated
by `fornax-large-alloc::zeroed_samples`, and returns the existing
`CroppedMosaic<T>` wrapper. The public owned-return types are unchanged.

`Primitive` now uses the narrower `Zeroable` requirement needed by the
allocator, rather than the stronger `Pod` requirement. The supported primitive
set remains `f32` and `f64`.

## Exact Equality

CLI TIFF output before and after:

| Input | SHA-256 |
| --- | --- |
| 5D3 uncompressed | `28915a249f3e79a2880a20333e424b057d044e3757697a3de190abd43d89053c` |
| Sigma fp 4K | `92bde2f4ef681ae5b5245f8de52329483435197ad88d42b1e36a4b302ba1b8af` |

The same hash was produced before and after on both inputs.

## Crop Stage

Three fresh processes by five interleaved rounds at one Rayon worker. Values
are median `[min-max]` and include decode plus crop/cast to `f32`.

| Input | Metric | Plan tip | Shared helper | Change |
| --- | --- | ---: | ---: | ---: |
| 5D3 uncompressed | crop/cast ms | 110.580 `[106.876-153.690]` | 44.073 `[37.717-166.281]` | -60.1% |
| 5D3 uncompressed | minor faults | 29772 `[29683-31563]` | 5335 `[5293-10295]` | -82.1% |
| 5D3 uncompressed | peak RSS KiB | 187768 `[187620-188016]` | 184004 `[175816-184260]` | -2.0% |
| Sigma fp 4K | crop/cast ms | 41.210 `[40.489-43.093]` | 18.000 `[16.087-39.133]` | -56.3% |
| Sigma fp 4K | minor faults | 12075 `[12032-12103]` | 3705 `[3340-4369]` | -69.3% |
| Sigma fp 4K | peak RSS KiB | 65168 `[65036-65296]` | 34064 `[33296-65104]` | -47.7% |

The medians improve by more than the per-stage 20% gate. A few candidate
processes show higher maxima because THP backing can vary between fresh
processes; all median fault counts are substantially lower.

## Policy Test

`crates/fornax-raw-core/tests/crop_cast.rs`:

- verifies half-open crop bounds and exact conversion;
- checks all seven adapter sources delegate to the shared helper;
- checks no adapter reintroduces a local `fn crop_and_cast`;
- checks the helper calls `zeroed_samples`.

The allocator's own tests continue to own the unsafe counting allocator, so no
unsafe test code was added outside `fornax-large-alloc`.

## Validation

Passed:

- `cargo test -p fornax-raw-core --test crop_cast`
- `cargo test --workspace --all-features --exclude compare-demosaic`
  (682 passed, 3 ignored)
- `cargo check` for all seven adapters plus `fornax-raw-core` and
  `fornax-large-alloc` on Windows MSVC and macOS arm64
- `fornax-large-alloc` checks with `large-page-hint` off on both targets
- `git diff --check`

Miri is not installed for the nightly toolchain. Full workspace
`--all-features` remains blocked by the uninitialized `compare-demosaic`
reference sources; the suite above excludes that crate.

## Unsafe

This item adds no unsafe code. All production unsafe remains in the two
existing blocks in `fornax-large-alloc`. The pre-existing macOS
`sysctlbyname` unsafe block in `fornax-raw-core::utils` is untouched and outside
this item's scope.
