# fornax-cr3

Canon CR3/CRM BMFF parsing, CMT/CTMD metadata mapping, and lossless CRX Bayer
decoding. The crate accepts only conventional 2x2 RGB Bayer files whose CMP1
header matches Fornax's audited lossless scope. C-RAW, previews, HEIF tracks,
auxiliary raw tracks, and ambiguous layouts return typed errors.
