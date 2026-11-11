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

## Targeted reference digests

The following integer mosaics were decoded once with Rawler 0.8.0 in an
out-of-workspace tool. Fornax hashes the same little-endian `u16` sample
sequence and matches exactly.

| Corpus path | Dimensions | SHA-256 |
| --- | ---: | --- |
| `Canon/EOS R5/Canon_EOS_R5_RAW_ISO_100_nocrop_nodual.CR3` | 8352x5586 | `2660f27211bbe9322195ba1533e928c3f1de6b8bf654f7dd2c41e3839b007b10` |
| `Canon/EOS R6 Mark II/canon-r6m2-img_0346-raw-normal-full.cr3` | 6188x4120 | `fa67690c7bdc448e146808f54176a456af25459c42cfcccb1539649333e00b83` |

`Canon/EOS R5/Canon_EOS_R5_CRAW_ISO_100_nocrop_nodual.CR3` and
`Canon/EOS R6 Mark II/canon-r6m2-img_0361-craw-normal-full.cr3` classify as
`unsupported-lossy`.
