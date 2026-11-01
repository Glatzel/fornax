# compare-demosaic: agent guide

Compares Fornax demosaic output with LibRaw and darktable. The baseline JSON is
the only source of truth for results.

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

## Files

- `baseline/canonical.json`: accepted results, one line per row. Never edit by
  hand.
- `README.md`: hand-written text, generated regions between compare-demosaic
  markers, and note blocks. Never edit text between markers by hand.
- `target/compare-demosaic/`: latest run (not committed). `--out` changes it.

## Investigating one algorithm or case

```bash
compare-demosaic run --algorithm ahd --reference libraw --pattern RGGB \
  --artifacts --out target/dbg
compare-demosaic diff crates/compare-demosaic/baseline/canonical.json \
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

## Failure investigation order

1. Tail or whole image? Compare `p999`, `count_gt1024`, `max` with `mae`.
2. Border? Compare `all` with `interior`.
3. Where? Open the error map and the tile summary.
4. Which channel or site type? `channels`, `sites`.
5. Same split in other algorithms? A shared cause beats a per-algorithm fix.
6. Only then diff intermediate stages against the reference build.

## Updating the baseline

```bash
run
cp target/compare-demosaic/canonical.json crates/compare-demosaic/baseline/canonical.json
readme
```

Review `git diff`, fill in any TODO notes, commit. If local numbers differ from
CI, copy `canonical.json` from the CI artifact instead.

## Writing note blocks

Every algorithm needs a note with Reference, Why this reference, Why we match,
Known differences, and optionally Open issues. Explain causes and conventions
(shared steps, constants, rounding, borders), never current results or numbers.
Reference functions by name, not path. Remove Open issues when fixed.

## CI

One job: cargo test, `readme --check`, `run --artifacts`, diff against the
baseline. On any failure the artifact `compare-demosaic` holds canonical.json,
diff.txt and the artifacts. If nothing differs, nothing is uploaded.

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
