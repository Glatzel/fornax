# Notation

Define each symbol here before its first use in a chapter. Keep definitions
consistent with the implementation and cite the source that fixes the
convention.

## Samples and coordinates

| Symbol | Meaning | Source |
| --- | --- | --- |
| $I(x,y)$ | Single-channel sample at integer column $x$ and row $y$ | [Core image types](../../crates/fornax-raw-core/src/lib.rs) |
| $I_R$, $I_G$, $I_B$ | Reconstructed red, green, and blue channels | [Postprocessor API](../../crates/fornax-raw-pipeline/src/lib.rs) |
| $M$ | Bayer mosaic before linear RGB reconstruction | [Architecture](../../docs/architecture/overview.md) |
| $b$ | Black level in input sample codes | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |
| $w$ | White level in input sample codes | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |
| $x_n$ | Normalized sample | [Preprocessor](../../crates/fornax-raw-pipeline/src/lib.rs) |

The normalization described by the default Fornax preprocessor is:

$$
x_n = \max\left(0, \frac{x - b}{w - b}\right)
$$

The equation assumes $w > b$ and applies after black-level correction and
clipping; see [the preprocessor source](../../crates/fornax-raw-pipeline/src/lib.rs).

## Labels and metrics

| Symbol | Meaning | Source |
| --- | --- | --- |
| $G_1$, $G_2$ | The two green sites in a 2x2 RGB Bayer repeat | [VNG notes](../../docs/reference/algorithms/vng.md) |
| MAE | Mean absolute error between corresponding samples | [Comparison guide](../../crates/compare-demosaic/README.md) |
| RMSE | Root mean square error between corresponding samples | [Comparison guide](../../crates/compare-demosaic/README.md) |
| P99 | 99th percentile of absolute sample error | [Comparison guide](../../crates/compare-demosaic/README.md) |
| `RGGB`, `BGGR`, `GRBG`, `GBRG` | Supported 2x2 Bayer phase names | [Bayer patterns](../../crates/fornax-raw-core/src/lib.rs) |

Add symbols only when a chapter or reference note needs them. Do not redefine
a symbol locally in chapter prose.
