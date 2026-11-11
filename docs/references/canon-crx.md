# Canon CRX reference notes

## Pinned behavior

- Canon CR3/CRM container notes:
  <https://github.com/lclevy/canon_cr3>.
- dnglab/Rawler at
  `62b77de1a206d467e910d55199191068b45a9cc1`, files
  `rawler/src/decoders/cr3.rs`, `rawler/src/formats/bmff/ext_cr3/`, and
  `rawler/src/decompressors/crx/`.
- LibRaw at `cf02d66eb5c19d4e46df7aee0582e47a4918509a` for behavioral
  comparison only.

Rawler and LibRaw are LGPL references. Fornax does not copy their source into
production crates or add either as a production dependency.

## Lossless discriminator

The targeted corpus audit found the following full-resolution RAW CMP1 shape:

| Field | Accepted lossless value |
| --- | --- |
| Version | `0x0100` |
| Planes | `4` |
| `enc_type` | `0` |
| Wavelet levels | `0` |
| CFA layout | `0..=3` |
| MDAT header size | `0x70` |
| Bit depth | `8..=16` |

The targeted R5 and R6 Mark II C-RAW files instead use CMP1 version `0x0200`
with three wavelet levels. The decoder therefore accepts only the tuple above
and returns `DecodeError::Unsupported` for every other combination. This
intentionally treats an unobserved or ambiguous tuple as unsupported rather
than assuming it is lossless.

The full 110-file classification must be produced by the remote decoder
corpus workflow. RAW payloads are transient and are never committed or
uploaded.
