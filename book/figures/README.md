# Procedural book figures

Book figures are generated from Python source and never committed as assets.
Matplotlib lives in the isolated `book` pixi environment, so the default
development environment is unchanged.

## Generate figures

Run all chapter-prefixed generators from the repository root:

```bash
pixi run -e book book-figures
```

The runner discovers files named `lNN_<topic>.py` in this directory and runs
them in filename order. Output goes to
`book/src/figures/<chapter>/<name>.svg`, which Git ignores. The output has no
size limit; optimize the generator and the explanation instead of the SVG.

To test the helper and sample generators:

```bash
pixi run -e book python -m unittest book.figures.test_figures
```

## Current figures

| Chapter | Generator | Output | Alternative text |
| --- | --- | --- | --- |
| L01 | `l01_bayer_patterns.py` | `figures/l01/bayer-patterns.svg` | Four labeled Bayer color-filter arrays. RGGB starts with red at top left and blue at bottom right; GRBG starts with green, red, blue, then green; GBRG starts with green, blue, red, then green; and BGGR starts with blue at top left and red at bottom right. |
| L02 | `l02_level_normalization.py` | `figures/l02/level-normalization.svg` | A line plot of a digital sensor code mapped to a normalized value. The normalized value stays at zero below the black level of 512, rises linearly to one at the white level of 4095, and stays at one for larger codes. |

Each generator also exports the same text as `ALT_TEXT`. When the L01 or L02
chapter embeds its figure, copy that value into the Markdown image
description. Do not replace it with "figure" or repeat the caption.

## Authoring conventions

- Put one figure group in each `lNN_<topic>.py` script. Put reusable behavior
  and the palette in `common.py`.
- Call `configure_matplotlib()` and save with
  `save_svg(figure, "<chapter>", "<name>")`. The helper sets transparent SVG
  output, keeps labels as text, fixes the SVG hash salt, and removes the
  creation date.
- Keep the result pure vector. Never add `<image>`, `data:image`, base64 data,
  or a raster preview to an SVG.
- Use the shared mid-contrast colors. A figure must remain understandable
  when reduced to grayscale and must be legible on both light and dark book
  themes. Add labels, line styles, or shapes when color alone carries meaning.
- Give every figure descriptive alternative text that states what the figure
  demonstrates. Record it in the generator and in the table above.
- Keep formulas and labels as text, and use plain colors rather than a
  decorative background.
- Use the temporary `temp/book/` directory for light/dark previews or raster
  conversion. Delete or leave those previews untracked; never commit them.

The generated SVGs are deterministic on one machine when Matplotlib, fonts,
and script inputs are unchanged. Cross-platform byte identity is not required
because text layout depends on font metrics, so CI does not regenerate or
compare figures.
