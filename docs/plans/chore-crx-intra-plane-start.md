# Canon CRX intra-plane speculative-start gate

Status: design and production-path prototype complete; production speculative
code is not approved.

## Goal
Determine whether a row-band speculative start can raise CRX utilization beyond
the existing four plane tasks without changing decoded bytes or strict error
behavior.

## Measured work units
Each accepted file has one tile and four independently coded planes. Each plane
has one subband, and accepted lossless files have zero transform levels.

| File | Sample bytes | Planes: data bytes | Tile |
| --- | ---: | --- | --- |
| R5 | 35,744,104 | 8,852,236 / 9,364,150 / 9,366,342 / 8,161,251 | 8352x5586 |
| R6 | 7,072,120 | 1,763,970 / 1,818,346 / 1,820,223 / 1,669,451 | 3584x2386 |
| R7 | 28,640,024 | 7,094,789 / 7,374,054 / 7,369,344 / 6,801,719 | 7128x4732 |

The offsets and sizes are populated by
`crates/fornax-raw/src/decoders/canon/codec/mod.rs:180-268`; one
subband per plane is enforced at lines 227-253. Four plane tasks are created at
lines 135-169.

## Production prototype
Five-round, 16-thread runs of the current exact four-task path measured:

| File | Median decode | CPUs used |
| --- | ---: | ---: |
| R5 | 340.99 ms | 2.61 |
| R6 | 46.28 ms | 2.59 |
| R7 | 304.38 ms | 2.28 |

The R5 1T profile attributes 43.9% to Rice decoding, 47.5% to line
prediction/reconstruction, and 5.9% to placement. Placement is already fused
into the direct path at `codec/mod.rs:416-449`. Decoding and reconstruction
cannot be separated safely because Rice adaptation and MED context advance in
the same loop at `codec/mod.rs:594-700`.

## Checkpoint gate

A valid speculative start needs the exact bit position, Rice `k`, run parameter,
prior two reconstructed rows, and global row ordinal. CRX exposes no restart,
row-offset table, or stream checksum from which those values can be recovered.
Starting at a proportional byte offset can produce syntactically valid symbols
with the wrong state, so there is no independent verifiable checkpoint.

The worst-case retry is one partial band per worker followed by the full
sequential decode, bounding duplicate entropy work below one additional scan.
Corrupt input would be discarded until the sequential fallback returned its
original error, but speculative output cannot be accepted without a checkpoint.

## Decision

Stop before production speculation. A future design must expose a container
checkpoint or a trusted row-to-bit-position map whose construction does not
repeat the hot Huffman/Rice walk. Row-parallel placement remains rejected by
the measured neutral-to-negative result.
