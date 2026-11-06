# bench-decoder

Compares the default pure-Rust DNG decoder with the Rawler adapter through the
shared `IDecoder` contract.

The benchmark measures the five `IDecoder` methods:

- `decode`;
- `raw_mosaic`;
- `cropped_mosaic`;
- `dng_metadata`;
- `process_metadata`.

```bash
cargo bench -p bench-decoder
```

The default fixture is `external/raw-images/images/colorchart-5D2-6000K.dng`.
Select another file with:

```bash
FORNAX_BENCH_RAW=/path/to/file.dng cargo bench -p bench-decoder
```

`FORNAX_BENCH_BITS` selects `32`, `64`, or a comma-separated list. The default
is `32`.

`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, or a comma-separated list.
An unset or empty value benchmarks both:

```bash
FORNAX_BENCH_DECODER=fornax FORNAX_BENCH_BITS=32 \
  cargo bench -p bench-decoder
```

The benchmark prints each decoder's DNG metadata tag count before timing its
metadata accessor. Compare metadata timings only when both decoders expose
equivalent tag coverage.

Compile the benchmark without running timings:

```bash
cargo bench -p bench-decoder --no-run
```
