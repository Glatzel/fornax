# Nikon NEF format plan

**Tier:** 1

**Status:** Partially implemented for standard uncompressed/packed modes `2`,
`5`, `6`, `7`, `9`, and `10`. Huffman lossless/lossy modes, HE/HE*, small raw,
reduced-linear output, Coolpix-specific layouts, and headerless maker notes
remain unsupported.

## Container structure

NEF is a TIFF container with EXIF and Nikon maker-note IFDs. The raw image is
selected from the CFA IFD or the zero `NewSubFileType` IFD. Strips may be
uncompressed, packed, or Huffman-compressed. Geometry, compression mode,
white balance, crop, and levels are split between TIFF and maker notes.

## Compression schemes

Known Nikon compression modes are:

| Mode | Meaning |
| --- | --- |
| `1` | Lossy type 1 Huffman. |
| `2` | Uncompressed. |
| `3` | Lossless Huffman. |
| `4` | Lossy type 2 Huffman. |
| `5` | Striped packed 12-bit. |
| `6` | Uncompressed reduced 12-bit. |
| `7` | Unpacked 12-bit. |
| `8` | Small/reduced raw. |
| `9` | Packed 12-bit. |
| `10` | Packed 14-bit. |
| `13`, `14` | High Efficiency / HE*. |

The implemented first slice covers modes `2`, `5`, `6`, `7`, `9`, and `10`.
Huffman modes `1`, `3`, and `4`; small raw mode `8`; and HE/HE* modes `13` and
`14` return `DecodeError::Unsupported`.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, `UniqueCameraModel`, serial, lens identity;
- `ActiveArea` and `DefaultCropOrigin/Size`;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`;
- `BlackLevelRepeatDim`, `BlackLevel`, `WhiteLevel`;
- `AsShotNeutral` from WhiteBalance/ColorBalance maker-note coefficients;
- `Orientation`, exposure, and safe EXIF values.

Nikon does not always expose a white level in the raw IFD; derive it from bit
depth and camera metadata only when the format's sample interpretation supports
that derivation.

## Known variants and pitfalls

- TIFF and maker-note endianness can disagree; reject mismatch as a typed
  unsupported variant or corrupt structure rather than guessing.
- Uncompressed 12/14-bit samples may be packed, unpacked, shifted, or
  row-padded depending on camera and compression mode.
- Compressed strips may be contiguous even when several `StripOffsets` exist;
  iterate strip metadata rather than assuming one layout.
- Small/reduced-resolution NEF can produce three-component linear data, which
  is not a CFA mosaic.
- White-balance coefficient versions differ across generations.
- Some camera models override sampled width or row structure.
- HE/HE* use a substantially different codec and must not be mislabeled as
  ordinary lossless NEF.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/nef.rs`, Nikon maker-note/WB modules, packed
  decompressors, and lossless Huffman decoder.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Nikon compression and level behavior.

## Definition of done

- Every implemented NEF compression mode has at least one accepted corpus row
  with `pass=1`.
- Supported integer samples match Rawler or LibRaw exactly after crop, CFA,
  endianness, packing, and WB normalization.
- HE/HE* and linear/reduced modes return typed unsupported results unless
  explicitly brought into scope.
- Synthetic tests cover packed/unpacked 12/14-bit rows, row padding, Huffman
  mode dispatch, maker-note endianness, and Bayer handoff.
