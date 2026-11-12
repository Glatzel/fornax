# The Fornax RAW Pipeline Book

This book explains how a camera RAW file becomes RGB pixels, using the
pipeline implemented by Fornax as its concrete model. It is written for a
photographer who wants the ideas without assuming an image-processing
background, and for a student or engineer who wants the equations,
derivations, and primary references.

Fornax keeps decoding, preprocessing, demosaicing, and post-processing as
separate pipeline concerns; see the
[architecture documentation](../docs/architecture/overview.md). The book follows those
boundaries and treats conventional 2x2 RGB Bayer input as the active demosaic
target.

```mermaid
accTitle: Fornax RAW pipeline stages
accDescr: Decode produces a mosaic and metadata, preprocessing normalizes the active mosaic, demosaicing reconstructs RGB, and post-processing applies white balance and output work.
flowchart LR
    decode["Decode<br/>RAW file to mosaic and metadata"]
    preprocess["Preprocess<br/>normalize the active mosaic"]
    demosaic["Demosaic<br/>Bayer mosaic to RGB"]
    postprocess["Post-process<br/>white balance and output"]

    decode --> preprocess
    preprocess --> demosaic
    demosaic --> postprocess
```

The current `PostProcessor` API composes demosaicing with its RGB output
work. The diagram separates them because the book teaches reconstruction and
post-processing as distinct ideas.

## Reading paths

- Photographers can start with L00, L01, and L10, then choose demosaic
  chapters by the visual artifacts they want to understand.
- Students and engineers can read the chapters in order while using
  [Notation](src/notation.md) as the symbol reference.
- Maintainers should read [the book working agreement](AGENTS.md) before
  drafting or reviewing a chapter.

## Chapter status

A chapter starts only after the behavior it describes is implemented and
merged. **Ready** means prerequisites are met and a chapter may be drafted;
it does not mean a chapter exists. Status values are planned, ready, draft,
rewrite, and done. Only the maintainer marks a chapter done.

| ID | Chapter | Prerequisite | Status |
| --- | --- | --- | --- |
| L00 | Pipeline overview | Decode and later stages exist | Ready |
| L01 | From photons to numbers | A decoder and Bayer metadata exist | Ready |
| L02 | Preprocess | Preprocessor and level metadata are merged | Ready |
| L10 | The demosaicing problem | One demosaic implementation exists | Ready |
| L11 | Interpolation basics | Linear demosaicing is merged | Ready |
| L12 | Using channel correlation | One correlation-based implementation is merged | Ready |
| L13 | Direction-adaptive methods | VNG and PPG are merged | Ready |
| L14 | Homogeneity-directed methods | AHD is merged | Ready |
| L15 | Statistical and iterative methods | LMMSE, DCB, AMaZE, DHT, and AAHD are merged | Ready |
| L16 | Artifacts and evaluation | `compare-demosaic` is usable | Ready |
| L30 | Post-process chapters | Post-processing scope is split into chapter items | Planned |

The ordered chapter map is maintained in [SUMMARY](src/SUMMARY.md). No empty
chapter files are committed. Algorithm results and reference comparisons
belong in [the comparison guide](../crates/compare-demosaic/README.md), not in
this status table.

## Repository context

- [Architecture](../docs/architecture/overview.md) defines stage interfaces and crate
  responsibilities.
- [Algorithm notes](../docs/reference) preserve implementation research.
- [Book bibliography](src/bibliography.md) records external sources and
  licence constraints.
- [Book glossary](src/glossary.md) defines terms used across chapters.

All non-Markdown assets follow [the binary-asset rule](AGENTS.md). Figures are
generated into Git-ignored paths, and temporary book work belongs under
`temp/book/`.
