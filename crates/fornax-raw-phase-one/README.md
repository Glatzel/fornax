# fornax-raw-phase-one

Pure-Rust Phase One and Leaf IIQ decoding for Fornax.

The crate parses classic TIFF plus the Phase One private maker note, resolves
offsets relative to the private base, decodes IIQ modes 0, 3, 5, 6, and 8, and
applies the source sensor-correction directory before returning U16 samples. It
maps active-area, crop, 2x2 CFA, corrected levels, white balance, and camera
identity into `fornax_tiff::Directory` and `ProcessMetadata`.

IIQ uncompressed, IIQ-L/L16, nonlinear, and IIQ-Sv2 decoders live in this crate
behind a private `ICodecDecode` codec value.

Legacy Raw1/Raw2 modes, unknown camera profiles, Leaf `.MOS`, Phase One `.TIF`,
preview extraction, and non-2x2 pipeline input return typed unsupported errors.

Behavioral provenance and licensing constraints are recorded in
[Phase One IIQ references](../../docs/reference/decoders/phase-one-iiq.md).
