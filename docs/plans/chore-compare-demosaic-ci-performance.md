# Compare-demosaic CI performance

## Goal

Reduce the clean-run duration of the `compare-demosaic` CI job while preserving
byte-identical canonical output, full crate coverage, and diagnostic artifacts
for baseline regressions.

The measured baseline is a 5m23s job with 5m08s in `Test, run, compare`.
The log timeline spends about 2m34s between the duplicate debug crate test and
the README check, then about 2m12s before the first comparison row. The causes
are a duplicated workspace test, separate debug and fat-LTO release builds,
unconditional generation of 48 full-resolution PNG artifact sets, and global
serialization of all reference demosaic calls.

## Scope

In scope:

- Reuse the existing Ubuntu workspace test matrix for compare-demosaic tests.
- Build the release binary once and invoke it directly in CI.
- Generate artifacts only when the canonical baseline diff fails.
- Record build, README, run, and diff wall times in CI logs.
- Permit concurrent reference execution except for LibRaw AAHD.
- Add a regression test covering concurrent execution of every comparison.
- Document the revised CI behavior and record the measurement.

Out of scope:

- Changing comparison limits, baseline values, generated README regions, or
  demosaic implementations.
- Adding workflow jobs, matrices, subcommands, or binary artifacts.
- Changing canonical JSON fields or artifact contents.

## Task checklist

- [ ] Write the expanded jobs=1 versus jobs=4 concurrency test and observe it
  fail or expose the correctness risk before narrowing reference locking.
- [ ] Narrow reference serialization to LibRaw AAHD and make the concurrency
  regression test pass.
- [ ] Update CI to build once, invoke the release binary directly, emit phase
  timings, and generate artifacts only after a failed baseline diff.
- [ ] Update the compare-demosaic agent guide with the new CI ownership and
  artifact behavior.
- [ ] Run the full required validation and record outcomes.
- [ ] Open one semantic pull request with this plan.

## Files expected to change

- `.github/workflows/ci.yml`
- `crates/compare-demosaic/AGENTS.md`
- `crates/compare-demosaic/src/reference.rs`
- `crates/compare-demosaic/src/run.rs`
- `docs/plans/chore-compare-demosaic-ci-performance.md`

## Validation

Run from the worktree root:

```bash
source scripts/setup.sh
cargo test -p compare-demosaic --all-features
cargo build --release -p compare-demosaic
target/release/compare-demosaic readme --check
target/release/compare-demosaic run
target/release/compare-demosaic diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
git diff --check
```

Record local phase times. Confirm the clean run produces a byte-identical
canonical report and does not create artifacts. Confirm a mismatched diff
takes the artifact-regeneration path. Confirm the first clean GitHub Actions
comparison step is below the 5m08s baseline.

## Decisions and notes

- The Ubuntu `test-build` matrix remains the source of crate test coverage;
  duplicating it in the comparison job wastes a separate debug and release
  dependency graph.
- Clean runs skip artifact PNGs. A failed diff reruns the comparison with
  `--artifacts`, preserving the existing failure upload and developer
  diagnostics at the cost of a second run only on regression.
- LibRaw links `raw_r`, whose separate instances are thread-safe, and darktable
  wrappers use per-call buffers. LibRaw AAHD has a mutable static gamma lookup
  table and remains serialized.
- No CLI or schema changes are needed.
