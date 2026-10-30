# bench-decoder

Measures complete RAW decoding through the Fornax Rawler and LibRaw adapters
with `f32` and `f64` output samples.

```bash
cargo bench -p bench-decoder
```

Set `FORNAX_BENCH_RAW` to benchmark another RAW file. The default fixture is
installed by `python3 scripts/setup.py`.
