# Decoder References

Decoder notes are grouped by their primary purpose:

- [perf](perf/) contains benchmarks, measurements, profiles, and optimization
  investigations.
- [design](design/) contains architecture decisions, trade-offs, and
  implementation gates or plans.
- [reference](reference/) contains format, decoder, API, field-table, and
  specification research.

Files that mix concerns remain intact under their dominant category.
`reference/flashraw.md` is primarily the benchmark-adapter contract but also
records benchmark behavior. `perf/dng-remaining-gap.md` is primarily a
performance report but also records allocator design decisions.

## Performance

- [Sony ARW remaining performance gap](perf/arw-remaining-gap.md)
- [ARW gap evidence](perf/arw-gap-profiles/)
- [DNG performance experiment log](perf/dng-experiments-log.md)
- [DNG remaining performance gap](perf/dng-remaining-gap.md)
- [DNG gap profile index](perf/dng-gap-profiles/README.md)
- [DNG gap resume notes](perf/dng-gap-profiles/RESUME.md)
- [FlashRaw 0.2.0 CPU source comparison](perf/flashraw-cpu-comparison.md)

## Design

- [Restart-free single-scan LJPEG parallelism](design/restart-free-single-scan-parallelism-design.md)

## Reference

- [Canon CRX and CR3](reference/canon-crx.md)
- [DNG compression](reference/dng-compression.md)
- [DNG and lossless JPEG-92](reference/dng-lossless-jpeg.md)
- [DNG opcodes](reference/dng-opcodes.md)
- [DNG output color](reference/dng-output-color.md)
- [FlashRaw benchmark adapter](reference/flashraw.md)
- [Fujifilm RAF](reference/fuji-raf.md)
- [Hasselblad 3FR](reference/hasselblad-3fr.md)
- [Nikon NEF](reference/nikon-nef.md)
- [Panasonic RW2](reference/panasonic-rw2.md)
- [Phase One IIQ](reference/phase-one-iiq.md)
- [Sony ARW/NRW](reference/sony-arw.md)
