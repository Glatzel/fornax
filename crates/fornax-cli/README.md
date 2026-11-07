# fornax-cli

Simple batch RAW converter for the Fornax pipeline.

```bash
cargo run -p fornax-cli -- photo.dng
cargo run -p fornax-cli -- --format tiff --output converted raw-folder
cargo run -p fornax-cli -- --format jpeg --jobs 1 --metadata-json photo.dng
```

## Defaults

| Setting | Default |
| --- | --- |
| Output format | DNG |
| Output directory | `converted` |
| Decoder | Auto: Fornax for DNG, Rawler for other `RawFormat` inputs |
| Jobs | 1 |
| Recursive directory scan | On |
| Existing output | Skip |
| DNG compression | Lossless JPEG |
| Demosaic | `Demosaic::default()` with default parameters |
| White balance | On |
| Camera-to-linear-sRGB matrix | On |
| JPEG quality | 90 |
| Metadata JSON | Off |

## Outputs

- DNG output preserves the raw mosaic and metadata. DNG-to-DNG jobs are
  skipped; `--metadata-json` can still emit a sidecar.
- TIFF output is 16-bit linear RGB.
- JPEG output is an 8-bit preview using the standard linear-to-sRGB transfer.
- `--metadata-json` writes `<output-stem>.json`.

TIF and JPEG are documented preview/output conveniences, not a complete DNG
camera-profile renderer.

## Configuration

`--config` accepts TOML or JSON. Paths in the file resolve relative to the
config file. Explicit CLI options override the file.

```toml
inputs = ["raws"]
output = "converted"
format = "jpeg"
decoder = "auto"
jobs = 1
recursive = true
overwrite = false
metadata_json = true
demosaic = "rcd"
white_balance = true
color_matrix = true
jpeg_quality = 90
dng_compression = "lossless-jpeg"
zstd_level = 3
```

Directory inputs are scanned recursively for extensions recognized by
`fornax::RawFormat` and mirrored beneath
`<output>/<input-root>/<relative-path>`.
