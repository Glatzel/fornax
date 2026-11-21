# Canon CRX Parallelism Design Gate

Reduce R5/R6/R7 wall time only if another exact work unit exists without unsafe aliasing or doubled full-plane memory.

## Work units and dependencies

- Each real file contains one lossless tile and four independently coded planes.
- Each plane contains exactly one subband with known offset and byte size.
- Accepted lossless files have zero wavelet levels, so there is no inverse-wavelet row or level task to expose.
- Within a plane, Rice `k` and MED line context are sequential. A line needs the preceding reconstructed line, and the next symbol depends on adaptive Rice state.
- Tiles cannot add parallelism because each file has one. Subbands cannot add parallelism because each plane has one.
- Tile tasks and plane tasks therefore provide the same maximum of four leaf tasks. Direct one-thread placement avoids temporary planes; the parallel path retains four planes and interleaves them afterward.

## Speculative plane splitting

A row-band worker would need both the adaptive Rice state and reconstructed MED context. CRX has no restart marker or context checkpoint, so a false bit alignment can remain syntactically valid while producing different predictors. The stream has no checksum that proves an arbitrary start is the true symbol boundary. Without a checkpoint, speculative starts cannot satisfy the requirement that corrupt inputs return the same result or error as sequential decoding.

## Prototype

Throwaway scaling probe, release profile, one warmup batch:

| File | 1T ms | 2T ms | 4T ms | 8T ms | 16T ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| R5 | 924 | 595 | 372 | 422 | 386 |
| R6 | 163 | 98 | 66 | 70 | 68 |
| R7 | 719 | 479 | 333 | 351 | 351 |

Four threads is the knee. Eight and sixteen threads add no throughput because only four independent plane tasks exist. CPU utilization around 2.2-2.5 reflects unequal task duration and post-decode integration, not an available fifth exact task.

## Recommendation

Do not land a speculative CRX row/band design from the current format evidence. A safe next design would require a verifiable stream checkpoint or a new parser representation that separates run/context decisions from reconstruction, neither of which fits the Phase 2 prototype box.
