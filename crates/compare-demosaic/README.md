# compare-demosaic

Compares Fornax demosaic output with LibRaw and darktable at the demosaic
stage. The committed baseline is the source of truth for pass/fail results.
Generated tables are in [RESULTS.md](RESULTS.md).

## Run

```bash
source scripts/setup.sh
cargo run --release -p compare-demosaic
cargo run --release -p compare-demosaic -- --algorithm ahd
cargo run --release -p compare-demosaic -- --check
cargo run --release -p compare-demosaic -- --update
```

The default run writes `target/compare-demosaic/canonical.json`. `--check`
exits 1 on a regression, missing row, new row, or metadata mismatch.
`--update` replaces the baseline and regenerates `RESULTS.md`; review the git
diff before committing it.

## Limits

Robust rows pass when all three limits hold:

- MAE at most 2 codes.
- P99 absolute error at most 64 codes.
- At most 0.001 of samples above 1024 codes.

All errors are 16-bit codes. Exact rows pass only when every sample matches.
Rounding rows allow the one-code upward difference caused by Fornax rounding
to nearest while LibRaw truncates.

## Algorithm notes

- AAHD diverges strongly across the tested patterns; the two green-first
  label pairs are materially closer than the opposite diagonal.
- AHD is close on the green-first labels and diverges on the red-first and
  blue-first labels, with errors concentrated in discrete decisions.
- AMaZE exceeds the robust limits on every pattern.
- DCB exceeds the robust limits on every pattern; the green-first labels are
  closer than the opposite diagonal.
- DHT remains materially divergent even on its closest label pair.
- Linear differs only because Fornax rounds to nearest while LibRaw truncates.
- LMMSE fails its headline limits on every pattern.
- PPG is bit-identical to darktable PPG. Against LibRaw it passes with
  isolated outliers.
- RCD stays within every robust limit and retains a small outlier tail on the
  red-first and blue-first labels.
- VNG shows broad failures against both references.

The other Bayer labels relabel the same mosaic and serve as consistency checks;
prioritise rows using the input's native pattern.
