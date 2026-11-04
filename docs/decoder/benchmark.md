# Rawler performance comparison

## Goal

Keep `fornax-decoder` in the same performance class as Rawler on a small,
stable set of representative files. The target is a Fornax/Rawler median decode
time ratio between 0.80 and 1.25. The comparison is diagnostic, not a
correctness gate and not a command to beat Rawler.

## Scope

The benchmark measures pure decode to the common raw model:

- file open and container parse;
- compressed payload decode;
- metadata collection needed for `DecodedRaw`;
- no preprocess, demosaic, Python conversion, preview decode, or DNG write.

The fixed set contains one accepted DNG, CR3, ARW, and NEF sample from the
corpus manifest. Tier 2 samples are added only when that format starts. Files
are identified by SHA-256 and downloaded through the corpus tool; binaries are
not committed.

Rawler remains a development-only dependency of the benchmark crate. It must
not appear in the `fornax-decoder`, `fornax-core`, `fornax-preprocessor`, or
`fornax-postprocessor` dependency trees.

## Implementation

Extend `crates/bench-decoder` rather than creating comparison infrastructure:

1. Keep the existing Criterion harness.
2. Add a benchmark group for the fixed corpus set.
3. Run both decoders on the same file and in the same process.
4. Use a warm-up iteration to page the file into cache, then report the median
   wall-clock time from at least five samples.
5. Use `decode_full` semantics for both implementations. If the native API
   returns integer samples and Rawler returns converted floats, keep sample
   conversion outside the timed region or report both decode-only and
   converted-output timings.
6. Print a per-file ratio and a geometric mean over the set.
7. Allow `FORNAX_BENCH_RAW_DIR` to override the download root and
   `FORNAX_BENCH_FORMATS` to filter local runs.

Do not add a separate timing service, profiler integration, corpus database, or
host-specific baselines.

## Pass/fail use

The benchmark reports ratios and does not fail at the 1.25 boundary until the
first tier-1 implementation has stable measurements. After that:

- mark a performance regression when the median ratio exceeds 1.25 on the
  same machine and corpus revision;
- investigate a ratio below 0.80 for accidental work omission or cache effects
  before treating it as an improvement;
- never hide a large single-file regression with the geometric mean.

Results are local/CI diagnostics and are not committed as a baseline because
host variance makes them volatile. A human review can record the command,
host class, corpus revision, and ratios in the pull request.

## Definition of done

- The benchmark builds with Rawler only as a dev dependency.
- The fixed set resolves only through the corpus manifest.
- DNG, CR3, ARW, and NEF each have a reported Fornax/Rawler ratio.
- The report separates read/parse/decode from any sample-type conversion.
- The dependency-tree check proves no Rawler dependency in the production
  decoder crate.
