# Large Allocation for Downstream Pipeline Stages

This plan records the pre-change audit, stage measurements, and review gate for
using the large-page policy outside `fornax-codec`.

## Goal

Remove avoidable full-image allocations in the rendering pipeline first, then
decide with measurements whether the existing large-page allocation policy
should be shared with later stages.

## Scope

In scope:

- Audit full-image allocations from decoded samples through CLI output.
- Measure cumulative stage timing and minor faults on a 5D3-class file and one
  smaller file.
- Estimate how much render time is attributable to fresh large allocations.
- Identify portable allocation removals before considering huge-page hints.

Out of scope until separate approval:

- Production code changes.
- Moving `large_alloc.rs`.
- Exposing a new public allocation API.
- Caller-owned decode buffers or any other public API change.
- SIMD, target features, or non-portable pipeline compute.

## Baseline

- Branch: `chore/large-alloc-pipeline-stages`
- Base: `origin/main` at `3b7729b`
- Host: WSL2 Debian, Ryzen 7 5800H, 15 visible CPUs
- Kernel: `6.6.87.1-microsoft-standard-WSL2`
- glibc: `2.41-12+deb13u3`
- THP: `enabled=[madvise]`, `defrag=[madvise]`
- Cargo: 1.100.0
- perf: 6.12.111
- Worker count: one Rayon worker
- Inputs: page-cache hot

The measurement used a temporary release-mode example probe. It was removed
after measurement, so this review gate contains no production or test changes.

## Full-Image Allocation Audit

Sizes below are observed or calculated for the 5D3 run. The probe observed a
93,536,000-byte cropped `f32` buffer, an 89,722,080-byte normalized buffer, and
a 269,166,240-byte RGB output.

| Crate | File:line | Element/output | Approx. 5D3 size | Runs | In-place or cheap-waste opportunity |
| --- | --- | --- | ---: | ---: | --- |
| `fornax-dng` | `dng_decoder/mod.rs:181` | Cropped and type-converted mosaic | 89.7-93.5 MB | 1 | Crop already runs in place; type conversion needs a new buffer. The same helper is duplicated in all seven decoder adapters. |
| `fornax-raw-pipeline` | `preprocessor.rs:35` | Normalization input | No new full buffer | 1 | Already uses `into_raw`, truncates to geometry, and normalizes in place. Keep. |
| `fornax-raw-pipeline` | `demosaic/protocol.rs:36` | Final `RgbImage::new` | 269.2 MB | 1 | Required output, but currently allocated with the standard allocator. |
| `fornax-raw-pipeline` | `rcd.rs:40` | PPG seed image consumed by RCD | 269.2 MB | 1 | Owned intermediate. It should be moved into the final RGB result instead of copied. |
| `fornax-raw-pipeline` | `postprocessor.rs:77` | `collect`/`collect_with` result | 269.2 MB | 1 | For owned demosaic results this duplicates the intermediate. An internal consume/move path can remove it without changing the public owned-return API. |
| `fornax-raw-pipeline` (AMaZE) | `amaze.rs:70`, `amaze.rs:148`, `buffer.rs:74` | Normalized input, RGB workspace, dense output | 89.7 MB + 2 x 269.2 MB | 1 each | `f32` workspace conversion can be fused or moved for `T=f32`; generic fallback remains. |
| `fornax-raw-pipeline` (AHD) | `ahd.rs:77`, `ahd.rs:82`, `ahd.rs:86` | Raw scale, RGB scale, final RGB | 89.7 MB + 2 x 269.2 MB | 1 each | Linear seed and scaled output are avoidable copies; tile scratch is not full-image. |
| `fornax-raw-pipeline` (AAHD) | `aahd.rs:92`, `aahd.rs:118`, `aahd.rs:133` | Two RGB planes, two YUV planes, flags, output | Greater than 1 GB peak | 1 each | Algorithm scratch; no simple in-place reduction. Not the default path. |
| `fornax-raw-pipeline` (DCB) | `dcb.rs:62`, `dcb.rs:129`, `dcb.rs:201`, `dcb.rs:629` | Four-channel pixels and candidate/chroma planes | Greater than 1 GB peak | 1 each | Multiple full scratch planes; not the default path. |
| `fornax-raw-pipeline` (DHT) | `dht.rs:61`, `dht.rs:89`, `dht.rs:545` | Three-channel working image, directions, output | About 426 MB before conversion | 1 each | Scratch is intrinsic to DHT; conversion can be separated as a candidate. |
| `fornax-raw-pipeline` (LMMSE) | `lmmse.rs:52`, `lmmse.rs:57`, `lmmse.rs:60` | Codes, PPG input, seed, output | About 493 MB peak | 1 each | Multiple conversion copies; not the default path. |
| `fornax-raw-pipeline` (VNG) | `vng.rs:202`, `vng.rs:218` | RGB `u16` and four-channel pixels | About 314 MB | 1 each | Large scratch cannot be removed trivially; conversion to `f32` remains. |
| `fornax-raw-cli` | `convert.rs:231` | TIFF `Rgb<u16>` collect | 134.6 MB | At most 1 | Output encoding currently materializes a second full image. |
| `fornax-raw-cli` | `convert.rs:252` | JPEG `Rgb<u8>` collect | 67.3 MB | At most 1 | Could stream rows if the encoder supports it. |
| `fornax-dng` | `sample_buffer.rs:7` | Clone for DNG writing | 46.8 MB U16 | At most 1 | DNG re-encode path, not render; ownership can be moved instead of cloned. |
| `fornax-dng` | `dng_decoder/transforms.rs:68` | Deinterleaved image | 46.8 MB U16 | Conditional | This full-image decode transform still bypasses `zeroed_samples`. |
| `fornax-dng` | `dng_decoder/transforms.rs:137` | OpcodeList1 working/output copies | 46.8-93.5 MB each | Conditional | Multiple full-image transform outputs bypass `zeroed_samples`. |
| `fornax-dng` | `dng_decoder/transforms.rs:339` | Linearization output | 46.8 MB U16 | Conditional | Full-image output bypasses `zeroed_samples`. |
| `fornax-raw-py` | `lib.rs:64` | Legacy RGB buffer | Not runnable | N/A | The crate is excluded and still imports the removed `fornax` API; it is not part of the current runnable pipeline. |

The seven decoder adapters duplicate `crop_and_cast`: DNG, Canon, Sony,
Hasselblad, Nikon, Panasonic, and Fujifilm. The DNG measurement represents the
same allocation shape, but all seven call sites need coverage before enforcing
a policy test.

## Cumulative Stage Timing

Three fresh processes, five interleaved rounds per process. Milliseconds are
median `[min-max]`. Each mode starts from a new decoder value; `render` is the
end-to-end CLI render path with the default RCD demosaic.

| Input | Mode | Wall |
| --- | --- | ---: |
| 5D3 uncompressed | decode | 18.650 `[17.085-39.802]` |
| 5D3 uncompressed | crop/cast | 101.814 `[98.726-131.384]` |
| 5D3 uncompressed | preprocess | 111.630 `[107.234-143.760]` |
| 5D3 uncompressed | demosaic | 2857.127 `[2798.406-3185.340]` |
| 5D3 uncompressed | render | 2851.590 `[2792.889-3217.131]` |
| Sigma fp 4K | decode | 6.244 `[5.759-20.245]` |
| Sigma fp 4K | crop/cast | 10.202 `[9.247-43.514]` |
| Sigma fp 4K | preprocess | 13.692 `[12.447-36.187]` |
| Sigma fp 4K | demosaic | 1013.744 `[979.978-1055.440]` |
| Sigma fp 4K | render | 1012.532 `[992.393-1056.384]` |

The crop and demosaic endpoints are the allocation-heavy points. Preprocessing
adds about 10 ms on 5D3 and 3.5 ms on Sigma while reusing its input storage.
The final render endpoint is indistinguishable from the demosaic endpoint in
these cumulative runs, so postprocessing is not a separate large wall-time
stage.

## Minor Faults

Median of three fresh one-decode processes measured with
`perf stat -e minor-faults`.

| Input | Decode | Crop/cast | Preprocess | Demosaic | Render |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 1,837 | 24,675 | 24,684 | 156,172 | 156,172 |
| Sigma fp 4K | 1,369 | 9,545 | 9,569 | 58,660 | 58,662 |

Incremental faults correspond closely to output sizes:

- 5D3 crop adds 22,838 pages, or 93.54 MB.
- 5D3 demosaic/render adds 131,488 pages, or 538.57 MB, matching the PPG
  intermediate plus the copied final RGB image.
- Sigma crop adds 8,176 pages, or 33.49 MB.
- Sigma demosaic/render adds 49,091 pages, or 201.08 MB, matching two RGB
  outputs.

## Fresh-Allocation Share

To avoid changing production code, glibc was configured during measurement to
keep large allocations on a reusable heap:
`MALLOC_MMAP_MAX_=0 MALLOC_TRIM_THRESHOLD_=-1`. This proxy removes repeated
large mapping and first-touch work while still writing every output sample.

| Input | Default render | Heap-reuse proxy | Difference | Share of render |
| --- | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 2911.100 ms `[2824.236-3040.442]` | 2246.685 ms `[2202.835-2914.420]` | -664.415 ms | 22.8% |
| Sigma fp 4K | 1024.156 ms `[981.505-1121.806]` | 838.772 ms `[827.061-1067.361]` | -185.384 ms | 18.1% |

This is an upper-bound proxy for first-touch and fresh-mapping cost, not proof
that the Linux huge-page hint alone will recover the whole difference.

## Review Gate Decision

The allocation total is not within noise. The next change should be the
portable ownership fix before any allocator move:

1. Add an internal consume/move path for owned lazy-demosaic images.
2. Use it from `PostProcessor` so the default RCD path does not allocate a
   second 269.2 MB RGB buffer.
3. Apply white balance or color transforms in place when possible.
4. Re-run exact output equality, wall time, and faults.

Only after that measured result should the module-location decision be made.
The default recommendation remains moving the single module to the lowest
shared crate, but no location is approved yet.

## Task Checklist

- [x] Audit full-image allocations and bypasses.
- [x] Measure cumulative stage wall time.
- [x] Measure cumulative minor faults.
- [x] Estimate the fresh-allocation share with a no-code proxy.
- [x] Remove the temporary probe.
- [ ] Obtain approval for the Step 2.1 gate and the ownership candidate.
- [ ] Implement and measure the owned-demosaic move path.
- [ ] Report cheap-waste results and the module-location decision.

## Validation

At this gate:

- `git diff --check`: passed in the temporary-probe worktree after removing the
  probe.
- Current production/test diff: none.
- Temporary probe build: passed.
- Three-way decoder equality is unaffected because this gate contains no code.

After an approved implementation:

- Decoder three-way equality.
- Pipeline output byte equality before/after.
- `cargo test --all-features`.
- Counting-allocator tests extended to every new element type.
- Windows and macOS checks with the feature on and off.
- Miri fallback check if available.
