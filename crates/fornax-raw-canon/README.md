# fornax-raw-canon

Canon CR3/CRM BMFF parsing, CMT/CTMD metadata mapping, and lossless CRX Bayer
decoding. The crate accepts only conventional 2x2 RGB Bayer files whose CMP1
header matches Fornax's audited lossless scope. C-RAW, previews, HEIF tracks,
auxiliary raw tracks, and ambiguous layouts return typed errors.

The lossless CRX decoder and its CMP1 header and CFA parameter types live in
this crate behind a private `ICodecDecode` codec value.
