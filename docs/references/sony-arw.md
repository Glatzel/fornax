# Sony ARW/NRW reference notes

## Sources

- In-tree implementations:
  `fornax-codec/src/arw/{arw1.rs,arw2.rs,arw6.rs,ljpeg.rs}`.
- dnglab/Rawler at commit
  `62b77de1a206d467e910d55199191068b45a9cc1`, especially
  `rawler/src/decoders/arw.rs`,
  `rawler/src/decompressors/packed.rs`, and Sony camera metadata. License:
  LGPL-2.1; used as a behavioral reference only.
- LibRaw at commit `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, especially
  `src/decoders/decoders_dcraw.cpp` and `src/metadata/tiff.cpp`. License:
  LGPL-2.1 or CDDL-1.0; used as a behavioral reference only.
- TIFF 6.0, for classic TIFF directory and strip/tile mechanics.

## Container behavior

- `DNGPrivateData` contains a four-byte little-endian offset to the outer Sony
  private IFD. The outer IFD points through `0x7200`/`0x7201`/`0x7221` to an
  encrypted private TIFF. Decrypted value offsets remain relative to the
  original container, so `fornax_tiff::parse_ifd_at` applies a negative source
  offset correction when parsing the extracted buffer.
- The full-resolution CFA IFD is selected by image area, excluding reduced
  subfiles. `LinearRaw` and multi-sample payloads do not enter the Bayer
  handoff.
- ARW1 adds eight stored rows when the compressed byte count covers
  `width * (height + 8)`. Active-area metadata trims those rows.
- ARW2 is recognized by a `32767` strip whose byte count is exactly
  `width * height`, even when the TIFF `BitsPerSample` tag contains a historical
  16-bit value.

## Sample behavior

- ARW1 is one column-major JPEG-style difference stream decoded from right to
  left. Even rows precede odd rows within each column.
- ARW2 stores 16 same-colour samples per block. Values come from 11-bit
  extrema and 7-bit deltas, are doubled into a 4096-entry curve, then receive
  the deterministic per-row Sony dither. Rawler and Fornax match exactly on
  the selected ARW2 corpus fixtures.
- ARW6 is decoded by Fornax's independently written LLVC3 packet, wavelet, and
  plane-reconstruction implementation. Decoded codes are mapped through Sony's
  fixed 12-to-16-bit output curve.
- Sony lossless-JPEG CFA tiles commonly carry four components at `256x256`.
  Components `0/1/2/3` for each sample become `R/G` on the first 512-wide
  output row and `G/B` on the second. The Sony adapter in `fornax-codec` expands
  those four components into two Bayer rows.

## Metadata mapping

- `SonyRawImageSize` supplies the active area when present.
- Private tags `0x7303` and `0x7313` supply Sony white-balance gains; `0x7310`
  or `0x7300` supplies black levels; `0x787f` supplies the white level.
- `SonyRawCurve` (`0x7010`) supplies ARW2 curve knots.
- The source Make, Model, orientation, and non-structural tags are retained.
  `DNGPrivateData`, structural strip/tile tags, and stale offsets are omitted
  from regenerated metadata.

## Known differences and limits

- ARQ pixel-shift samples, reduced-resolution YCbCr `LinearRaw`, unknown
  compression families, and non-Bayer layouts return `DecodeError::Unsupported`.
- Recent Sony medium and small lossless-compressed RAW modes can use
  three-component, 15-bit YCbCr lossless JPEG tiles. Rawler exposes these as
  `LinearRaw` with `cpp = 3`, black level `1024`, white level `15360`, and
  identity white balance. Fornax deliberately keeps them unsupported because
  the current mosaic interface cannot represent multi-component output.
- Camera color matrices are not part of the ARW container. Fornax does not copy
  the LGPL Rawler camera database, so no matrix is synthesized when the source
  omits one.
- Crop fallbacks currently cover the validated DSLR-A700, NEX-5, and ILCE-7M3
  profiles. Other cameras use their full active area when the file has no
  explicit default-crop tags.

## LLVC3 curve provenance

The 4096-entry LLVC3 output table is fixed Sony format data, not an algorithm
implementation. Its values were checked against `flashraw-codecs` 0.1.1,
published under `MIT OR Apache-2.0`, and against the pre-migration decoded
sample hash for the ILCE-7M5 ARW6 fixture. Fornax carries the table as data and
does not adapt code from that crate. The implementation logic is independently
written from the documented packet and lifting behavior and validated against
the fixture output.

## Expected reduced-resolution YCbCr failures

The following corpus files are intentionally tracked as
`decode:unsupported`; they are not compression-decoder regressions:

- Sony ILCE-7M4:
  - `Sony/ILCE-7M4/ILCE-7M4_DSC06675_FullFrame-LossLess-Compressed-Medium.ARW`
  - `Sony/ILCE-7M4/ILCE-7M4_DSC06676_FullFrame-LossLess-Compressed-Small.ARW`
  - `Sony/ILCE-7M4/ILCE-7M4_DSC06680_APS-C-LossLess-Small.ARW`
- Sony ILCE-7CR:
  - `Sony/ILCE-7CR/DSC00796.ARW`
  - `Sony/ILCE-7CR/DSC00797.ARW`
  - `Sony/ILCE-7CR/DSC00800.ARW`
- Sony ILCE-9M3:
  - `Sony/ILCE-9M3/DSC00113.ARW`
  - `Sony/ILCE-9M3/DSC00114.ARW`
  - `Sony/ILCE-9M3/DSC00118.ARW`
- SONY ILCE-1M2:
  - `SONY/ILCE-1M2/A1_07628_lossless_compressed_medium_full_frame.ARW`
  - `SONY/ILCE-1M2/A1_07629_lossless_compressed_small_full_frame.ARW`
  - `SONY/ILCE-1M2/A1_07633_lossless_compressed_small_apsc.ARW`
- SONY DSC-RX1RM3:
  - `SONY/DSC-RX1RM3/DSC00255.ARW`
  - `SONY/DSC-RX1RM3/DSC00256.ARW`
