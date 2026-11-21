# Nikon Huffman Speculative Parallel Decode

Reduce the single-strip Nikon Huffman gap at 8T/16T without changing output or corruption behavior.

## Stream and predictor dependencies

- Every audited Nikon Huffman file has one strip and no restart markers.
- One Huffman code produces one signed difference in raster order.
- Predictor state uses the prior row of the same parity for columns 0-1 and
  otherwise the same row two columns left.
- A serial pre-pass therefore needs only the first two predictor states of each
  row. Once those are known, every remaining sample in a row is horizontally
  independent and can be curve-mapped in parallel.
- Fourteen-bit signed differences fit in `i16`, so workers can write their bit
  patterns into the final `u16` output and the predictor pass can overwrite
  them in place.

## Speculation

1. Split the bit stream into bounded segments, with a minimum segment size so
   small files remain sequential.
2. A worker starts at most 8 KiB before its target and decodes Huffman symbols
   within the target window, recording every symbol-end bit position.
3. Workers run independently and return differences plus boundary records.
4. The merge accepts a worker only when its recorded boundary exactly equals
   the previous worker's accepted end. It discards symbols decoded before that
   boundary.
5. If no exact boundary exists in the bounded window, or a worker reports an
   entropy error, decode that segment sequentially. The sequential path remains
   authoritative for both output and error.

## Correctness

An accepted worker starts exactly where the previous segment ends, so it sees
the identical Huffman byte/bit sequence as sequential decoding. The predictor
pre-pass computes the two row-start states serially; the row pass then applies
only the adjacent-horizontal recurrence. Truncation, invalid differences, and
curve overflow return the same typed error through sequential fallback.
Speculation never zero-pads or treats padding as a final sample.

## Memory and cost

The prototype stores no second full-resolution plane: differences and final
samples share one `u16` allocation. Extra memory is `O(width + height)` plus
per-worker boundary records. Failed speculation wastes at most one 16 KiB
window per segment before fallback.

## Prototype result

Release-profile prototype, real files, 8 KiB resync window:

| File | Resync at 16T | Skipped symbols | Serial ms | Prototype 16T ms |
| --- | ---: | ---: | ---: | ---: |
| Z5 II | 15/15 | 942 | 32.4 | 15.3 |
| D850 | 15/15 | 982 | 199.7 | 70.2 |
| Z7 | 15/15 | 998 | 232.6 | 68.0 |
| Z9 | 15/15 | 1002 | 227.3 | 71.8 |

At 8T the prototype measured 29.8, 83.9, 83.0, and 85.1 ms respectively.
Production implementation remains gated on this review.
