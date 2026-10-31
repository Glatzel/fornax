# compare-demosaic: agent guide

Compares Fornax demosaic output with LibRaw and darktable. The baseline file is
the only source of truth for results.

## Commands
```bash
cargo run --release -p compare-demosaic                     # run, write target/compare-demosaic/
cargo run --release -p compare-demosaic -- --algorithm ahd  # only AHD (repeatable); investigation only
cargo run --release -p compare-demosaic -- --check          # compare with baseline; exit 1 on regression
cargo run --release -p compare-demosaic -- --update         # overwrite baseline and RESULTS.md
```

Exit codes: 0 ok, 1 regression, 2 error.

## Files
- baseline/canonical.json: accepted results, one line per row. Never edit by hand.
- RESULTS.md: generated. Never edit by hand.
- README.md: hand-written, no numbers.
- target/compare-demosaic/: latest run output (not committed).

## Reading a row
All errors are in 16-bit codes. `verdict` is pass or fail against the limits in
`meta.limits` for the row's `kind` (exact, rounding, robust).

- `max`, `p99`, `p999`: tail of the absolute error. `p99` small but `mae` large
  means a few huge errors (decision flips), not a global offset.
- `mean_signed`: positive means Fornax is higher than the reference.
- `count_gt1024`, `count_gt1024_border`: large errors in total and inside the
  border strip. If they are equal, the problem is border handling.
- `changed_frac`: share of samples that differ at all.
- Prioritise rows with the file's `native_pattern`; the other patterns are the
  same data relabelled, so they are consistency checks.

## Investigating a failure
1. Read the row: is it the tail (`p999`, `count_gt1024`) or the whole image?
2. Compare `count_gt1024` with `count_gt1024_border`.
3. Run the example for detail:
   ```bash
   cargo run --release -p compare-demosaic --example inspect -- --algorithm ahd
   ```
4. Check whether other algorithms show the same pattern split (shared cause).
5. Only then diff intermediate stages against the reference build.

## CI
One job: `cargo test`, then `--check`. The artifact `compare-demosaic` holds the
latest `canonical.json` and `RESULTS.md`, even when the check fails.

To update the baseline: run `--update` locally, review `git diff`, and commit.
If local numbers differ from CI, copy `canonical.json` and `RESULTS.md` from the
CI artifact.

## Rules for changing this crate
- Do not add subcommands, flags, or workflow jobs. The CLI has four flags.
- Do not add generated files other than canonical.json and RESULTS.md.
- Do not add volatile fields (time, commit, host, hash) to canonical.json.
- Do not loosen `meta.limits` to make a row pass.
- New features must name the failure they prevent. Investigation helpers go in
  `examples/`, not `src/`.
- If a run is too slow, measure the step times in the Actions UI first. Only then
  consider parallelism, and write down the measurement in the PR.
