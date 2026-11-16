# R5: Decode Target Decision

## Result

The decode-target implementation was prototyped and dropped. It does not meet
the no-regression gate, so this branch contains measurements only and no
public API change.

The prototype added a `DecodeTarget::{Native, F32, F64}` option to
`DecodeRequest`, decoded integers/floats directly into the target at the row
sink, used direct target decoding for transform-free DNGs, cropped the decoded
float image in place, and retained the R6 crop/cast path as a fallback for
transform-bearing DNGs. Value-preserving casts and f32 rounding tests passed.

## Crop Sink Decision

Synthetic 5D3-size `u16` input, 3 fresh processes x 5 rounds:

| Design | Cast time | Minor faults | Peak RSS |
| --- | ---: | ---: | ---: |
| Crop-aware cast | 85.667 ms | 179052 | 86084 KiB |
| Full f32 cast, then in-place crop | 47.478 ms | 140096 | 22033 KiB |

The crop-aware sink is not justified. If decode-target work is reopened, use
contiguous target conversion followed by in-place crop.

The fault-count difference is dominated by how the synthetic probe allocated
the full native input; it is not a claim that in-place crop has fewer faults.

## End-to-End Result

CLI TIFF render, three fresh processes by five rounds. Values are median
milliseconds.

| Input | Workers | R6b baseline | R5 prototype | Change | R5 minor faults |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 1 | 2982 | 2901 | -2.7% | 218409 -> 215705 |
| 5D3 uncompressed | 15 | 2972 | 2924 | -1.6% | 217492 -> 214916 |
| Sigma fp 4K | 1 | 1092 | 1119 | +2.5% | 82564 -> 90686 |
| Sigma fp 4K | 15 | 1099 | 1125 | +2.4% | 82826 -> 91119 |

The isolated crop/cast stage showed the cause more clearly:

| Input | R6b crop/cast | R5 target decode + crop | Change |
| --- | ---: | ---: | ---: |
| 5D3 uncompressed | 37.779 ms | 41.486 ms | +9.8% |
| Sigma fp 4K | 17.260 ms | 49.335 ms | +185.8% |

Direct target decoding bypassed the optimized native 12/14-bit unpack path and
made Sigma substantially slower. 5D3's small end-to-end improvement is below
the 20% stage gate.

## Equality

TIFF output remained byte-identical during the prototype:

| Input | SHA-256 |
| --- | --- |
| 5D3 uncompressed | `28915a249f3e79a2880a20333e424b057d044e3757697a3de190abd43d89053c` |
| Sigma fp 4K | `92bde2f4ef681ae5b5245f8de52329483435197ad88d42b1e36a4b302ba1b8af` |

## Decision

- Drop the decode-target public option.
- Keep the R6 shared crop/cast fallback.
- Do not add F16/F128 or a new request field.
- Revisit only if the native 12/14-bit unpack sink can emit the target type
  without losing its optimized kernel.

## Validation

- The prototype passed workspace tests excluding `compare-demosaic` (689
  passed, 4 ignored).
- The temporary target tests and probes were removed.
- The final branch has no production or test-code diff.
- `git diff --check`.

## Unsafe

No unsafe code was added. Production unsafe remains limited to the existing
allocator blocks.
