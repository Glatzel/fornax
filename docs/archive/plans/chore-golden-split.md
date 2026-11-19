# Golden test/perf split implementation plan

## Goal

Fix the three LJPEG correctness gaps, split the golden corpus into manual
correctness and performance manifests sharing one hash file, rewrite consumers,
and report the release-profile FP16 alignment result.

## Scope

- Fix Sony four-component fast-path gating, strict LJPEG exhaustion, and clear
  restart-interval rejection.
- Create `golden/test-manifest.csv` and `golden/perf-manifest.csv` with
  decoder-path, reason, and overhead metadata.
- Add a fast golden-set checker and extend hash checking for manifest subsets.
- Point correctness consumers at the test set and benchmark consumers at the
  perf set.
- Keep `golden/selected-manifest.csv` as the legacy three-column test alias for
  one release.
- Append release-profile FP16 A/B results to the existing performance report.

## Task checklist

- [x] Reproduce and remove the three ignored Sony tests.
- [x] Add real-bit watermarks to both LJPEG readers and DRI rejection.
- [x] Create and validate the two draft manifests.
- [x] Generate hashes for newly included files and cross-check raw output.
- [x] Add checker tests and implement `decoder-tracker golden-check`.
- [x] Rewire tests, benchmarks, probes, scripts, and CI defaults.
- [x] Update golden README and compatibility alias.
- [x] Run DNG/ARW regression sweep, golden hashes, focused speed checks.
- [x] Run release-profile FP16 interleaved A/B and append the report.
- [ ] Run workspace validation and hand off three new commits.

## Decoder path

The checker reuses the existing tier-0/1 lossless grouping:

- DNG groups by compression, sample type, bit depth, unpacking/kernel family,
  storage, predictor, and compression ID where applicable.
- ARW/CR3/NEF group by format, compression, sample type, storage, and
  predictor; ARW/NEF aggregate supported bit depths.
- Only fully passing, lossless/uncompressed baseline rows are eligible.

There are currently 18 covered paths.

## Decisions and notes

- Nonzero point transforms fall back to the generic LJPEG path.
- Valid entropy ending exactly after the final MCU is accepted. Consuming a
  Huffman code or amplitude bit beyond the last real bit is an exhaustion error.
- Restart intervals are rejected as typed `Unsupported`; predictor reset,
  alignment, and marker-state support is not small/local enough for this task.
- The draft manifests deliberately include small overhead-only Sigma rows only
  in the perf manifest.
