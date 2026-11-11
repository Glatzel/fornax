# L00 - Pipeline overview

> **Status:** Implemented for conventional 2x2 RGB Bayer input; other CFA
> layouts are not supported ([FornaxArchitecture](../bibliography.md#fornaxarchitecture)).

## 1. Why it matters

<!-- DRAFT -->
In Fornax's active pipeline, the RAW input supplies sensor measurements and the
metadata needed to interpret them, not a display-ready RGB image
([DNG1710](../bibliography.md#dng1710)). Fornax makes that distinction concrete
at decode, where the Rawler adapter converts a CFA payload into a single-channel
mosaic and common metadata
([rawler decoder:31-44](../../crates/fornax-decoder-rawler/src/lib.rs#L31-L44)).
That separation lets a file-format change stay in the decoder, a level change
stay in preprocessing, and spatial reconstruction stay in post-processing
([architecture:40-84](../../docs/architecture.md#L40-L84)).

The result is a pipeline of typed handoffs rather than one opaque conversion
([protocol:6-40](../../crates/fornax-core/src/protocol.rs#L6-L40)). The
repository example follows the same path from `decode_active` through
preprocessing and demosaicing for each enabled decoder
([pipeline example:35-55](../../crates/fornax/examples/pipeline.rs#L35-L55)).
This overview follows that path to show where a value changes stage and where
its meaning changes ([architecture:38-87](../../docs/architecture.md#L38-L87)).

## 2. The idea in one figure

<!-- DRAFT -->
Each arrow is a typed boundary with a defined input and output, not merely a
convention ([architecture:40-41](../../docs/architecture.md#L40-L41)).

```mermaid
accTitle: Fornax pipeline from RAW input to RGB output
accDescr: A RAW file is decoded to an active mosaic and metadata, preprocessed to a normalized mosaic, and reconstructed as RGB output.
flowchart LR
    raw["RAW input"]
    decoded["DecodedRawActive<br/>mosaic + Metadata"]
    normalized["RawImage<br/>normalized mosaic"]
    final["RgbImage<br/>reconstructed RGB"]

    raw -->|"decode and active-area crop"| decoded
    decoded -->|"black and white level normalization"| normalized
    normalized -->|"demosaic and RGB output work"| final
```

Decode returns an active mosaic with the metadata required by later stages
([rawler decoder:22-28](../../crates/fornax-decoder-rawler/src/lib.rs#L22-L28)).
Preprocessing preserves the single-channel layout while changing sample
meaning ([preprocessor:11-80](../../crates/fornax-preprocessor/src/lib.rs#L11-L80)).
The final boundary produces three channels per output pixel
([demosaic protocol:20-61](../../crates/fornax-postprocessor/src/protocol.rs#L20-L61)).

## 3. The math

<!-- DRAFT -->
At an active sensor site $(x,y)$, the mosaic contributes one measured value
$I(x,y)$ ([bayer patterns:44-79](../../crates/fornax-core/src/bayer.rs#L44-L79)).
The metadata supplies the site's Bayer color and the black and white levels
needed to interpret the measurement
([metadata:5-65](../../crates/fornax-core/src/metadata.rs#L5-L65)).
The decoder first restricts the stored frame to the active area
([rawler decoder:22-28](../../crates/fornax-decoder-rawler/src/lib.rs#L22-L28)).

The default preprocessor then normalizes each active-area sample using the
black level $b$ and white level $w$:

$$
x_n = \max\left(0, \frac{I(x,y) - b}{w - b}\right)
$$

Here the output is clamped at zero, and a non-positive denominator also yields
zero ([preprocessor:64-72](../../crates/fornax-preprocessor/src/lib.rs#L64-L72)).
The demosaic stage receives that normalized mosaic and the Bayer pattern, then
constructs red, green, and blue values
([demosaic interface:83-89](../../crates/fornax-postprocessor/src/protocol.rs#L83-L89)).

The stage type sequence summarizes the changing contracts:

```text
DecodedRawActive<T> -> RawImage<T> -> RgbImage<T>
```

The first boundary is preprocessing and the second includes reconstruction and
RGB output work ([architecture:63-84](../../docs/architecture.md#L63-L84)).

## 4. In Fornax

<!-- DRAFT -->
The `fornax` facade re-exports shared image and metadata types plus the decoder,
preprocessor, and postprocessor modules
([facade:1-7](../../crates/fornax/src/lib.rs#L1-L7)). The main stage traits live
in `fornax-core`, while demosaic's lazy row protocol lives in
`fornax-postprocessor`
([architecture:68-76](../../docs/architecture.md#L68-L76)).

| Stage | Owner | Input to output | Implemented responsibility |
| --- | --- | --- | --- |
| Decode | `fornax-decoder-rawler` and other decoder adapters through `IDecoder` | RAW input to `DecodedRawActive<T>` plus `Metadata<T>` | Decode a 2x2 RGB Bayer payload, crop the active area, and convert metadata ([rawler decoder:14-44](../../crates/fornax-decoder-rawler/src/lib.rs#L14-L44), [rawler metadata:319-330](../../crates/fornax-decoder-rawler/src/lib.rs#L319-L330)). |
| Preprocess | `fornax-preprocessor::Preprocessor` through `IPreprocessor` | `DecodedRawActive<T>` to `RawImage<T>` | Subtract the repeating black pattern and row or column deltas, divide by the remaining white range, and clamp negative values ([preprocessor:24-73](../../crates/fornax-preprocessor/src/lib.rs#L24-L73)). |
| Demosaic | `fornax-postprocessor::PostProcessor<D>` through `IDemosaic` | `RawImage<T>` to lazy RGB rows and then `RgbImage<T>` | Select a demosaic implementation from `D`, pass the Bayer pattern, and collect the reconstructed rows ([postprocessor:23-36](../../crates/fornax-postprocessor/src/lib.rs#L23-L36), [lazy output:20-59](../../crates/fornax-postprocessor/src/protocol.rs#L20-L59)). |
| RGB output work | `fornax-postprocessor::PostProcessor<D>` | demosaiced RGB to `RgbImage<T>` | Optionally apply green-normalized white balance after demosaicing; `PostProcessor::default()` enables this work with Linear demosaicing ([postprocessor:37-62](../../crates/fornax-postprocessor/src/lib.rs#L37-L62)). |

The Rust API composes demosaicing and RGB output work inside `PostProcessor`,
although the book treats them as separate ideas
([architecture:68-84](../../docs/architecture.md#L68-L84)). The default pipeline
does not apply `Metadata::default_crop` automatically; that crop remains a
recommendation for the caller
([architecture:86-87](../../docs/architecture.md#L86-L87)).

## 5. Failure modes

<!-- DRAFT -->
- A non-CFA payload, a payload with more than one sample per pixel, or a CFA
  whose repeat is not a 2x2 RGB pattern stops at decode
  ([rawler decoder:61-83](../../crates/fornax-decoder-rawler/src/lib.rs#L61-L83)).
  The accepted phase names are RGGB, BGGR, GRBG, and GBRG, matching Fornax's
  documented Bayer scope
  ([rawler metadata:268-279](../../crates/fornax-decoder-rawler/src/lib.rs#L268-L279),
  [architecture:89-94](../../docs/architecture.md#L89-L94)).
- An incorrectly aligned black-level tile changes normalization differently at
  each CFA site, producing channel-dependent output; the level pattern is
  applied from the top-left sample and repeats tile by tile
  ([metadata:44-48](../../crates/fornax-core/src/metadata.rs#L44-L48),
  [preprocessor:52-73](../../crates/fornax-preprocessor/src/lib.rs#L52-L73)).
- A caller that never applies `default_crop` retains the full active area,
  because the stage methods carry the crop recommendation without applying it
  ([architecture:86-87](../../docs/architecture.md#L86-L87)).
- Demosaic errors can concentrate near the border or in a small error tail;
  the comparison report separates complete-frame and interior statistics and
  records the worst sample coordinates
  ([comparison evaluation:128-142](../../crates/compare-demosaic/README.md#L128-L142)).

## 6. Try it

<!-- DRAFT -->
Run the existing end-to-end example with its default development RAW file
([pipeline example:35-37](../../crates/fornax/examples/pipeline.rs#L35-L37)):

```bash
cargo run -p fornax --example pipeline
```

The example writes a decoded mosaic, a normalized mosaic, and a demosaiced TIFF
for each enabled decoder under `temp/pipeline/`
([pipeline example:18-27](../../crates/fornax/examples/pipeline.rs#L18-L27),
[pipeline example:35-55](../../crates/fornax/examples/pipeline.rs#L35-L55)).
Open `2.preprocess.tiff` after `1.decoded.tiff` to see the level change without
spatial reconstruction
([pipeline example:18-23](../../crates/fornax/examples/pipeline.rs#L18-L23)).

To supply another conventional 2x2 Bayer RAW file, pass its path after `--`
([pipeline example:57-60](../../crates/fornax/examples/pipeline.rs#L57-L60)):

```bash
cargo run -p fornax --example pipeline -- path/to/input.cr2
```

## 7. Go deeper

<!-- DRAFT -->
Read the [Fornax architecture](../../docs/architecture.md) for the complete
stage and crate map ([FornaxArchitecture](../bibliography.md#fornaxarchitecture)).
The [Adobe DNG specification](../bibliography.md#dng1710) defines the raw-image
metadata concepts that Fornax maps into `Metadata`
([DNG1710](../bibliography.md#dng1710)). The
[comparison guide](../../crates/compare-demosaic/README.md) explains how output
is measured against reference implementations
([FornaxComparison](../bibliography.md#fornaxcomparison)). Continue with the
chapter order in [Summary](SUMMARY.md) after this overview
([chapter order:5-18](SUMMARY.md#planned-chapter-order)).

## 8. Check yourself

<!-- DRAFT -->
1. Which stage first produces three values per pixel, and what type does it
   return ([demosaic interface:83-89](../../crates/fornax-postprocessor/src/protocol.rs#L83-L89))?
2. What does the preprocessor use from metadata to normalize samples
   ([architecture:55-61](../../docs/architecture.md#L55-L61))?
3. Does demosaicing apply `default_crop` automatically
   ([architecture:86-87](../../docs/architecture.md#L86-L87))?
4. Which CFA layouts can reach Fornax's demosaic stage
   ([architecture:89-94](../../docs/architecture.md#L89-L94))?

<details>
<summary>Answers</summary>

1. Demosaicing produces RGB rows or an `RgbImage<T>` after receiving the
   normalized mosaic and Bayer pattern
   ([demosaic interface:83-89](../../crates/fornax-postprocessor/src/protocol.rs#L83-L89)).
2. It uses the black and white levels from `Metadata`, including the repeating
   black pattern and optional row or column deltas
   ([architecture:55-61](../../docs/architecture.md#L55-L61)).
3. No. The crop is carried as a recommendation and the current stage methods do
   not apply it ([architecture:86-87](../../docs/architecture.md#L86-L87)).
4. Only the four conventional 2x2 RGB Bayer phases are supported; X-Trans,
   four-color, monochrome, and other layouts are not
   ([architecture:89-94](../../docs/architecture.md#L89-L94)).

</details>
