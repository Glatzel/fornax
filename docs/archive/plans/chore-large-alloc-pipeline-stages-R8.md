# R8: Demosaic Allocation Cleanup

## Result

The shared helpers remove avoidable f32 copies while retaining the checked
generic conversion path:

- `normalized_f32` borrows an existing normalized f32 mosaic.
- `f32_codes_into` moves an owned dense f32 image into the integer-code domain.
- `dense_from_f32_owned` returns an owned f32 workspace directly.

Two algorithm commits passed the allocation/no-regression gate:

- AMaZE: borrowed f32 source and moved the f32 output.
- AHD: moved the linear RGB workspace and final f32 output.

AAHD's input clone removal was measured at 114.763 ms versus 115.065 ms
baseline, within noise, so that commit was dropped.

## Measurements

Synthetic 1000x700 f32 mosaic, three fresh processes by five interleaved
rounds at one worker. Values are median `[min-max]`.

| Algorithm | Baseline ms | Candidate ms | Change | Minor faults | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| AMaZE | 90.156 `[89.556-92.070]` | 87.239 `[86.767-89.082]` | -3.2% | 26928 -> 24196 | 43742 -> 39761 |
| AHD | 125.596 `[119.063-132.938]` | 117.303 `[116.183-126.718]` | -6.6% | 38016 -> 29824 | 66132 -> 56157 |
| AAHD | 115.065 `[114.365-142.852]` | 114.763 `[113.675-141.501]` | Reverted | 40548 -> 39156 | 58194 -> 56112 |

Byte equality hashes matched before/after:

| Algorithm | FNV hash |
| --- | --- |
| AMaZE | `ab925483023769bf` |
| AHD | `a9b9a84d30875034` |
| AAHD | `2e04cbaf0d62b28c` |

## Algorithm Audit

| Algorithm | Action | Reason |
| --- | --- | --- |
| AMaZE | Changed | Removed source and final f32 full-image copies. |
| AHD | Changed | Removed linear-workspace and final f32 copies. |
| AAHD | Reverted | Clone removal reduced faults but did not improve wall time beyond noise. |
| DCB | No change | Scratch candidate/chroma planes are intrinsic to the algorithm; conversion inputs and outputs are required. |
| DHT | No change | Padded `nraw`/direction maps are intrinsic scratch; no independent avoidable full-image copy remains. |
| LMMSE | No change | PPG seed conversion and `u16` interpolation workspace are required by its algorithm; no copy can be removed without changing the kernel. |
| VNG / VNG4 | No change | Linear seed and phase/gradient scratch are consumed by the algorithm; no standalone duplicate image remains. |
| PPG | No change | Writes rows directly from the borrowed source mosaic. |
| Linear | No change | Lazy source and collect pass already write directly into the final image. |
| RCD | Already handled by merged R4 | Owned final-image move and in-place postprocessing. |

## Validation

- `cargo test --workspace --all-features --exclude compare-demosaic`.
- Cross-target checks for `fornax-raw-pipeline` on Windows MSVC and macOS arm64.
- `git diff --check`.
- Temporary probe removed before the first commit.

## Unsafe

No unsafe code was added. Production unsafe remains limited to the existing
allocator blocks.
