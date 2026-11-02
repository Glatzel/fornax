# compare-demosaic: agent guide

Compares Fornax demosaic output with LibRaw and darktable. The baseline JSON is
the only source of truth for results. Repository-wide workflow lives in the
[root agent guide](../../AGENTS.md); this file covers only comparison work.

## Commands

```text
compare-demosaic run [--algorithm A]... [--reference R]... [--pattern P]... [--input F]...
                     [--out DIR] [--shard NAME] [--artifacts] [--border-px N]
                     [--jobs N] [--format text|json]
compare-demosaic diff <baseline.json> <new.json> [--verdict-only] [--format text|json]
compare-demosaic readme [--check] [--from FILE]
```

Run with `cargo run --release -p compare-demosaic -- <subcommand> ...`.
Exit codes: 0 ok, 1 differences or out of date, 2 error.

Common commands from the repository root:

```bash
cargo run --release -p compare-demosaic -- run
cargo run --release -p compare-demosaic -- run --algorithm ahd
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- run --artifacts
```

A full `run` writes `target/compare-demosaic/canonical.json`. A full `diff`
exits 1 for any difference, including a regression, missing or new row, or
metadata mismatch. `--verdict-only` ignores numeric-only changes when choosing
the exit code.

## Files

- `baseline/canonical.json`: accepted results, one line per row. Never edit by
  hand.
- `README.md`: hand-written text, generated regions between compare-demosaic
  markers, and note blocks. Never edit text between markers by hand.
- `target/compare-demosaic/`: latest run (not committed). `--out` changes it.
- From the repository root, query the 120 KB baseline with `jq -r '.rows[] | select(.verdict == "fail") | [.algorithm, .reference, .pattern, (.failed_limits | join(","))] | @tsv' crates/compare-demosaic/baseline/canonical.json` instead of reading it whole; read `README.md` by algorithm or marker section.

## Reference sources

LibRaw 0.22.3 reference headers and routines come from the repository-managed
`libraw-dev` Pixi package; do not compile LibRaw from the external checkout.
darktable reference code is pinned to the `5.6.2` release. Run
`scripts/setup.sh` on Linux/macOS or `scripts/setup.ps1` on Windows. The setup
scripts use `gh api` to materialize only the required darktable source files.
Keep the versions and commit URLs in `build.rs` and `src/cases.rs` synchronized
with those sources.

## Investigating one algorithm or case

```bash
cargo run --release -p compare-demosaic -- run --algorithm ahd \
  --reference libraw --pattern RGGB \
  --artifacts --out target/dbg
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/dbg/canonical.json
```

Partial runs record `meta.selection`; they cannot be used as a baseline.

## Reading a row (all errors are 16-bit codes)

- `all` and `interior` hold the same statistics; the difference is the border
  strip.
- `p99` small but `mae` large means a few huge errors (decision flips), not an
  offset.
- `mean_signed` > 0 means Fornax is higher than the reference.
- `sites.sampled` should be near zero error for most algorithms.
- `channels` shows which colour channel carries the error.
- `worst` lists the largest errors with coordinates and both values.
- Only `native` rows are independent tests. Other patterns use the smallest
  one-pixel crop that makes their CFA layout valid; `crop` records `[x, y]`.
- Artifacts:
  `<out>/artifacts/<input>/<algorithm>-<reference>-<pattern>-{error.png,tiles.json}`

## Accepted outliers

`⚖️` means every limit passes, high-error outliers remain, and a person
reviewed and accepted them. It is never derived from the numbers alone.

- Declare accepted outliers in the waiver table in `src/waivers.rs` with
  algorithm, reference, optional pattern and input, `max_count_gt1024`,
  `max_error`, mechanism, and evidence.
- Waive only rows that pass every limit. Fix a failing row or correct its test
  input; never waive a failure.
- Use the reviewed values as exact bounds. If a row gets worse, the symbol
  falls back to `⚠️` and `readme` exits 1 until someone reviews it again.
- Remove a waiver as soon as it is unused; `readme` reports unused waivers.
- Write what was checked as evidence, not merely that the result looks fine.

## Adding a comparison

Follow the
[add-demosaic-comparison skill](../../.agents/skills/add-demosaic-comparison/SKILL.md).

## Failure investigation order

1. Tail or whole image? Compare `p999`, `count_gt1024`, `max` with `mae`.
2. Border? Compare `all` with `interior`.
3. Where? Open the error map and the tile summary.
4. Which channel or site type? `channels`, `sites`.
5. Same split in other algorithms? A shared cause beats a per-algorithm fix.
6. Only then diff intermediate stages against the reference build.

## Updating the baseline

```bash
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json crates/compare-demosaic/baseline/canonical.json
cargo run --release -p compare-demosaic -- readme
```

Review `git diff`, fill in any TODO notes, commit. If local numbers differ from
CI, copy `canonical.json` from the CI artifact instead.

## Writing note blocks

Every algorithm needs a note with Reference, Why this reference, Why we match,
Known differences, and optionally Open issues. Explain causes and conventions
(shared steps, constants, rounding, borders), never current results or numbers.
Reference functions by name, not path. Remove Open issues when fixed.

## CI

The `compare-demosaic` job in `.github/workflows/ci.yml` runs the crate tests,
`readme --check`, `run --artifacts`, and a diff against the baseline. On failure
the `compare-demosaic` artifact holds `canonical.json`, `diff.txt`, and any
artifacts; if nothing differs, nothing is uploaded.

## Rules for changing this crate

- Three subcommands only: `run`, `diff`, `readme`. New debugging power goes
  into `run` options or the JSON statistics, not into new commands.
- No new workflow jobs, matrices, merge steps or binary artifacts.
- No generated files other than canonical.json and the generated README
  regions.
- No volatile fields (time, commit, host) in canonical.json.
- Never loosen `meta.limits` to make a row pass.
- A new algorithm needs a README section with a complete note;
  `readme --check` enforces it.
- Parallelism changes need a measurement of the step times first.
