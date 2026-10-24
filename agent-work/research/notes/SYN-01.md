# SYN-01 — Metadata and Sample Semantics Reconciliation

> **Task:** `SYN-01`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#540](https://github.com/Glatzel/fornax/pull/540).
> **Last updated:** 2026-10-24
>
> **Prerequisites:** `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06`, `RES-07`
>
> **Output consumed by:** `SYN-02`, `SYN-03`, design tasks `DES-001`–`DES-005`.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] A decision table states meaning, unit, coordinate convention, valid
>   range, supporting report sections, authority rule, and remaining
>   uncertainty.
> - [x] Disagreements and unresolved questions are explicit and not silently
>   resolved.
> - [x] Coordinate spaces and CFA alignment are defined precisely.
> - [x] Universal and backend-specific concepts are separated.
> - [x] Every conclusion links to a supporting report section.
> - [x] Implementation decisions explicitly weight LibRaw and dnglab as
>   high-priority references, subject to normative DNG rules.

## Scope and synthesis rules

This note reconciles only the merged findings in the prerequisite reports. It
does not introduce new upstream observations or finalize public Rust APIs.

The authority order is:

- **A1 — normative format semantics:** DNG 1.7 semantics govern DNG payloads.
- **A2 — verified backend behavior:** LibRaw and dnglab/rawler describe what
  their implementations actually emit. They are high-priority implementation
  references but do not override A1 for DNG.
- **A3 — backend provenance:** vendor-specific values supplied by a decoder,
  camera catalog, or file tag retain their source and confidence.
- **A4 — `fornax` invariants:** an adapter must reject a case it cannot
  translate without silently changing meaning.

When a backend has already collapsed or transformed source information, a
missing universal value is not evidence that the source value was zero. The
adapter must report the capability gap or reject the input before preprocessing.

## Decision table

| Item | Chosen meaning | Unit | Coordinate convention | Valid range | Supporting report sections | Authority rule | Remaining uncertainty |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Decoded samples | A row-major, single-plane sensor buffer whose samples remain in the backend's decoded source state. Decode does not claim that samples are black-corrected, linearized by a DNG table, or normalized unless provenance explicitly says so. | Backend-native sample code, or declared float sample unit | Stored-buffer `(row, col)`, origin at stored sample `(0,0)` | Representable range of the declared encoding; useful signal is expected to lie around black/white metadata | [RES-02 §2.2](../RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning); [RES-03 §2.1](../RES-03-dnglab.md#21-decode-representation-and-sample-domain); [RES-06 §2.3–§2.4](../RES-06-rust-raw-decoders.md#23-rawloader) | A1 for DNG; otherwise A2 plus A3 | Some LibRaw vendor decoders may unpack, scale, or apply format-specific corrections before exposing `raw_image`; fixtures must establish each target behavior. |
| Sample encoding | Typed unsigned-integer or IEEE-floating sample encoding separate from image dimensions and processing state. Integer bit depth is stored bit depth, not a normalized range. | Bits per sample; sample format enum | Encoding applies identically to every sample unless metadata explicitly declares otherwise | Unsigned integer `0..=2^bits - 1`; floating values bounded by their declared representation and metadata | [RES-02 §2.2](../RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning); [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-07 §2.1](../RES-07-rust-dng-tiff.md#21-image-rsimage-tiff) | A1 for DNG `BitsPerSample`/`SampleFormat`; otherwise A2 plus A3 | Signed integer, mixed per-plane formats, and every float bit depth were not established as initial targets. |
| Buffer geometry and stride | Buffer width and height identify stored samples. Row stride is a separate byte quantity and may exceed the tightly packed row length. | Width/height in pixels; stride in bytes | Stored-buffer origin `(0,0)`, top-left row first | `stride >= ceil(width * bytes_per_sample)` and the final row must fit in the allocation | [RES-01 §2.5](../RES-01-fornax-inventory.md#25-libraw-wrapper); [RES-02 §2.3](../RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates); [RES-06 §2.3–§2.4](../RES-06-rust-raw-decoders.md#23-rawloader); [RES-07 §2.1](../RES-07-rust-dng-tiff.md#21-image-rsimage-tiff) | A2 for the actual allocation; A4 for validation | The current LibRaw wrapper ignores `raw_pitch`; whether all target decoder outputs can be represented as one stride is unverified. |
| Stored, raw, and active dimensions | Stored-buffer dimensions are not aliases for active dimensions, selected dimensions, or default-crop dimensions. All are retained separately. | Pixels | Stored coordinates use the full buffer; active coordinates are derived from ActiveArea | Every active rectangle must lie inside the stored buffer | [RES-02 §2.3](../RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates); [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping) | A1 for DNG; A2 plus A3 for vendor RAW | Backend names such as `width`, `height`, and margins need adapter-specific mapping tests. |
| ActiveArea | A half-open rectangle `[top, bottom) × [left, right)` selecting non-masked samples in stored coordinates. It is not the display crop. | Integer sample coordinates | TLBR in stored-buffer coordinates; top and left refer to the included first active sample | `0 <= top < bottom <= stored_height`; `0 <= left < right <= stored_width` | [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.3–§2.4](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) | A1 for DNG; otherwise A3 | Vendor active-area conventions require fixture validation. |
| Default crop | Preserve `DefaultCropOrigin` and `DefaultCropSize` as crop metadata relative to ActiveArea, before `DefaultScale`. Rational values remain rational; integer-only backends must not silently truncate them. | Raw-image coordinates and dimensions before `DefaultScale` | Origin relative to ActiveArea top-left | Well-formed rectangle in the source coordinate model; fractional values allowed by DNG | [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping) | A1; A4 for exact representation | `DefaultScale` semantics and fractional-crop support remain open for design; rawler truncates to integer `Rect`. |
| Margins and MaskedAreas | Margins locate active data in the stored allocation. Masked rectangles are optional metadata and are not image pixels. Neither is inferred from the other. | Integer sample coordinates | Stored-buffer coordinates; each masked rectangle is half-open TLBR | Rectangles must be inside the stored buffer and should not silently overlap active image pixels | [RES-02 §2.3](../RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates); [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.3](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) | A1 for DNG; A3 for vendor | Masked-area black sampling is not dependable in the inspected darktable/rawspeed path and is not an initial black fallback. |
| CFA layout and phase | The initial universal layout is a conventional 2×2 RGB Bayer tile with four position identities `R`, `G1`, `G2`, `B`. Its phase is defined for stored samples as `((row - active.top) mod 2, (col - active.left) mod 2)`; ActiveArea remains stored separately. | CFA position identity | DNG tile origin is ActiveArea top-left; adapters may store a shifted lookup but must preserve the ActiveArea origin | Exactly 2 rows by 2 columns and conventional RGB plane mapping; all other layouts are rejected | [RES-01 §2.3](../RES-01-fornax-inventory.md#23-bayer-model); [RES-02 §2.4](../RES-02-libraw.md#24-cfa-representation-and-origin); [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-06 §2.3–§2.4](../RES-06-rust-raw-decoders.md#23-rawloader) | A1 for DNG; validated adapter mapping for vendor RAW; A4 rejection | X-Trans, four-color, monochrome, and nonstandard layouts are deferred. Vendor phase after format-specific cropping needs fixtures. |
| CFA plane mapping | Preserve `CFAPlaneColor` and `CFALayout`. The initial adapter accepts only the conventional RGB mapping; it must not assume that arbitrary CFA indices mean `R`, `G`, `B`. | Plane-color indices and layout enum | Relative to the declared CFA tile | Initial target: three RGB planes and standard rectangular layout | [RES-03 §2.2 and §4](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-06 §2.4](../RES-06-rust-raw-decoders.md#24-rawler) | A1; reject when A2 loses required tags | rawler's DNG path does not read these tags and assumes the common RGB mapping. |
| Orientation | Preserve TIFF orientation 1–8 as metadata for later presentation. Do not rotate or mirror the stored sensor buffer during decode. | TIFF orientation enum | Applies to the stored image as display orientation; sensor coordinates remain unchanged | `1..=8`; invalid values are rejected | [RES-02 §2.3](../RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates); [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping) | A1 for DNG; A3 for vendor | Vendor values after camera-specific transformation require fixture checks. |
| Common black offset | A scalar zero-light offset in stored sample units. It remains distinct from per-position offsets and repeat tiles even if preprocessing later combines them. | Stored sample codes | Applies to all stored sensor samples | Nonnegative and no greater than the selected white level | [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-04 §2.2–§2.3](../RES-04-darktable.md#22-rawprepare-inputs-and-output) | A1 for DNG; A2 plus A3 otherwise | LibRaw canonicalizes some scalar/per-channel representations before exposure. |
| Per-position black levels | Four offsets for the 2×2 Bayer positions `R`, `G1`, `G2`, `B`. A backend's four-value CFA array is evidence only after its index order and G-slot semantics are mapped. | Stored sample codes | CFA-position coordinates relative to ActiveArea top-left | Each offset is nonnegative and no greater than its applicable white level | [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-04 §2.2–§2.4](../RES-04-darktable.md#22-rawprepare-inputs-and-output); [RES-06 §2.4](../RES-06-rust-raw-decoders.md#24-rawler) | A1; A2 for index conversion | LibRaw `cblack[0..3]`, rawler's four-value reduction, and darktable/rawspeed's four slots need explicit mapping tests. |
| Repeat black tile | A typed row-by-column tile with per-sample-plane values. For conventional 1×1/2×2 Bayer tiles, an adapter may canonicalize values into common plus per-position black only if phase is preserved. Larger tiles remain representable and are rejected until preprocessing supports them. | Stored sample codes | Tile origin at ActiveArea top-left; row-major indices | Positive tile dimensions; value count equals rows × columns × planes | [RES-02 §2.5 and §5](../RES-02-libraw.md#25-black-level-representation); [RES-03 §2.2 and §5](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.2–§2.4](../RES-04-darktable.md#22-rawprepare-inputs-and-output) | A1; A4 rejection | LibRaw lacks an explicit tile-origin field; rawler's CFA scaling consumes only four values; darktable exposes no general tile. |
| Black row/column deltas | Preserve `BlackLevelDeltaH` and `BlackLevelDeltaV` as separate signed vectors when exposed. Absence is not converted to zero if the source may contain them but the adapter cannot read them. | Stored sample-code deltas, one per active column or row | Columns/rows relative to ActiveArea | Default is zero only when the source format explicitly supplies that default | [RES-03 §2.2 and §5](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-02 §5](../RES-02-libraw.md#5-open-questions-and-limitations) | A1; A2 capability report; A4 rejection if required | The inspected LibRaw path averages deltas and rawler does not parse them. Initial support status must be decided in `SYN-02`; silently dropping them is prohibited. |
| White level and saturation | Preserve the declared white level separately from a decoder-adjusted processing maximum and a measured data maximum. Normalized `1.0` is a preprocessing result, not an alias for the declared white level. | Stored sample codes; normalized value is dimensionless | Per sample plane in stored/active coordinates as supplied by the source | Declared unsigned default is `2^bits - 1`; declared float default is `1.0`; normalization requires white > black | [RES-02 §2.6](../RES-02-libraw.md#26-white-level-and-saturation); [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.2–§2.3](../RES-04-darktable.md#22-rawprepare-inputs-and-output) | A1 for declared DNG value; A2 plus A3 for adjusted/measured values | LibRaw `maximum` may be adjusted from observed data; not all backends expose measured maxima. |
| Linearization metadata | Preserve a linearization table in source order. Applying it maps stored codes into the linear source domain before black subtraction; decode without application remains stored-sample state. | Table entries in source output units; input index is stored code | One lookup domain over representable stored codes | Table index and output ranges must match the declared encoding | [RES-03 §2.1–§2.3](../RES-03-dnglab.md#21-decode-representation-and-sample-domain); [RES-04 §4](../RES-04-darktable.md#4-implications-for-fornax) | A1 | Whether a table is mandatory for every accepted DNG and how vendor linearization is exposed remain fixture questions; operation order belongs to `SYN-02`. |
| Normalized linear reference | The normalized representation is a distinct preprocessing output, not decoded source data. Its initial semantic target is DNG's linear reference model: zero-light maps to `0.0` and maximum useful signal maps to `1.0`. | Dimensionless normalized value | Same active sample grid as the corrected source; no origin reset | Intended `0.0..=1.0`; negative/overflow handling is an explicit `SYN-02` policy | [RES-03 §2.2 and §4](../RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.3–§2.4](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) | A1 for target meaning; A4 for explicit policy | DNG, rawler, and darktable disagree on clipping; no backend-specific policy is inherited. |
| Processing state | Stored, black-corrected, linearized, and normalized samples are distinct states. Correctness is represented structurally or by a non-forgeable state marker, not inferred from metadata being zero or from sample type alone. | State enum or distinct type invariant | State applies to the entire sample buffer and its active geometry | Exactly one recorded stage transition per operation; no double correction | [RES-01 §2.2](../RES-01-fornax-inventory.md#22-decoder-and-processor-contracts); [RES-02 §2.5 and §4](../RES-02-libraw.md#25-black-level-representation); [RES-03 §2.4 and §4](../RES-03-dnglab.md#24-processing-behavior-and-dng-opcode-boundaries); [RES-04 §2.7](../RES-04-darktable.md#27-relationship-to-libraw-and-dng) | A4 | Concrete representation is a design decision; metadata reset after LibRaw/rawler correction proves it cannot be the primary guard. |
| Metadata provenance and conflicts | Every typed universal value records its provider and origin. File tags, decoder catalogs, applied defaults, and measured values remain distinguishable. Conflicts are resolved by A1/A2/A3 above rather than by overwrite order. | Provider identifier, source location or tag, confidence/derivation | Provenance refers to the source coordinate/value just as the typed field does | Required for every consumed field; unknown source metadata may remain extensible | [RES-02 §2.7](../RES-02-libraw.md#27-metadata-retention-and-the-safe-wrapper); [RES-03 §2.5](../RES-03-dnglab.md#25-dng-versus-vendor-raw-metadata); [RES-06 §2.4](../RES-06-rust-raw-decoders.md#24-rawler) | A1 then A2 then A3; A4 rejects ambiguity | Exact key namespace and confidence vocabulary belong to `DES-002`. |
| Camera, WB, and color metadata | Preserve make/model, normalized names, white-balance coefficients, color matrices, and calibration metadata as descriptive data. They do not alter decode, preprocessing, or demosaic samples. | Backend-declared units; coefficients and matrices remain dimensionless metadata | Preserved with source coordinate/color-space meaning; not implicitly reprojected | Backend-declared; malformed or conflicting values are retained with provenance or rejected if required by the source model | [RES-01 §2.6](../RES-01-fornax-inventory.md#26-high-level-manager-and-python-bindings); [RES-02 §2.7](../RES-02-libraw.md#27-metadata-retention-and-the-safe-wrapper); [RES-04 §2.1 and §2.6](../RES-04-darktable.md#21-current-raw-pipeline-order) | A3 | Camera-catalog conflicts and matrix normalization conventions require `DES-002` and fixtures; WB must never be applied here. |
| DNG opcode metadata | Preserve opcode lists in their declared stage order with provenance. Do not execute an opcode merely because it is present; the accepted initial subset and stage behavior must be approved before implementation. | Opcode stage, identifier, parameters, and source order | Opcode coordinate domains remain those declared by the opcode/DNG spec | Only explicitly supported opcode/stage combinations; reject required unsupported opcodes | [RES-03 §2.4 and §5](../RES-03-dnglab.md#24-processing-behavior-and-dng-opcode-boundaries); [RES-07 §4](../RES-07-rust-dng-tiff.md#4-implications-for-fornax) | A1 for ordering; A4 for supported subset | Which OpcodeList1/2/3 entries are mandatory for the initial DNG corpus remains open. |
| Backend-specific state | LibRaw packed filters/`cblack`, rawler's mutable `RawImage`, backend rect types, tags, errors, and scaling methods are adapter internals. They never become canonical `fornax` concepts. | Backend-defined | Backend-defined | Must be translated or rejected at the adapter boundary | [RES-01 §2.5 and §4](../RES-01-fornax-inventory.md#25-libraw-wrapper); [RES-03 §4](../RES-03-dnglab.md#4-implications-for-fornax); [RES-06 §2.3–§2.5](../RES-06-rust-raw-decoders.md#23-rawloader); [RES-07 §2.2](../RES-07-rust-dng-tiff.md#22-rawlers-dng-facing-pieces) | A2 only as evidence; A4 for the universal boundary | Exact adapter trait remains `DES-006` work. |

## Conflict log

| Conflict | Source A | Source B | Resolution | Status |
| --- | --- | --- | --- | --- |
| When black correction occurs | LibRaw keeps `raw_image` until `raw2image`/`subtract_black`; [RES-02 §2.2](../RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning) | rawler returns stored samples but `apply_scaling()` combines linearization, black subtraction, and normalization; [RES-03 §2.1 and §2.4](../RES-03-dnglab.md#21-decode-representation-and-sample-domain) | Decode stops before either mutating correction path. Linearization and black correction are explicit later states. | Resolved |
| Black-level shape | DNG permits repeat tiles and row/column deltas; [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping) | LibRaw canonicalizes scalar/per-channel/repeat values; rawler and darktable consume four Bayer values. [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-04 §2.2](../RES-04-darktable.md#22-rawprepare-inputs-and-output) | Keep a general internal black model. Support only conventional 1×1/2×2 canonicalization initially; preserve or reject larger spatial forms. | Resolved for model; implementation gated |
| Black repeat origin | DNG anchors the tile at ActiveArea top-left; [RES-03 §2.2–§2.3](../RES-03-dnglab.md#22-dng-tag-mapping) | LibRaw's spatial subtraction uses cropped-image coordinates without a separate origin; [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation) | ActiveArea origin is authoritative. Reject a backend path that cannot preserve it after crop. | Resolved; backend behavior remains a test gate |
| Crop coordinate model | DNG separates stored, ActiveArea, and DefaultCrop spaces; [RES-03 §2.3](../RES-03-dnglab.md#23-coordinate-spaces-and-cfa-phase) | LibRaw exposes full dimensions, selected dimensions, and margins; darktable exposes crop borders. [RES-02 §2.3](../RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates); [RES-04 §2.4](../RES-04-darktable.md#24-cfa-indexing-and-crop-phase-preservation) | Keep all coordinate spaces as separate typed metadata; never collapse them into a single crop. | Resolved |
| White-level meaning | DNG declares per-plane saturation and a deterministic default; [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping) | LibRaw `maximum` may be adjusted toward observed data and also exposes `linear_max`/`data_maximum`; [RES-02 §2.6](../RES-02-libraw.md#26-white-level-and-saturation) | Preserve declared, adjusted, and measured values separately with provenance. | Resolved for representation |
| Clipping and overflow | DNG recommends clipping above `1.0` and permits retaining negatives; [RES-03 §2.2 and §2.4](../RES-03-dnglab.md#22-dng-tag-mapping) | rawler clips negatives but retains overflow; darktable `rawprepare` does not clip either side. [RES-03 §2.4](../RES-03-dnglab.md#24-processing-behavior-and-dng-opcode-boundaries); [RES-04 §2.3](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) | Do not inherit backend clipping. Defer the exact normalized clipping policy and tests to `SYN-02`. | Open — `SYN-02` |
| CFA plane mapping | DNG supplies `CFAPlaneColor` and `CFALayout`; [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping) | rawler's inspected DNG path assumes the common RGB mapping and does not read either tag. [RES-03 §4–§5](../RES-03-dnglab.md#4-implications-for-fornax) | Accept only conventional RGB Bayer initially. Reject alternative plane mappings or layouts. | Resolved |
| Black row/column deltas | DNG defines signed delta vectors; [RES-03 §2.2](../RES-03-dnglab.md#22-dng-tag-mapping) | LibRaw averages deltas into scalar black, while rawler declares but does not parse them. [RES-02 §5](../RES-02-libraw.md#5-open-questions-and-limitations); [RES-03 §5](../RES-03-dnglab.md#5-open-questions-and-limitations) | Preserve vectors when exposed; otherwise reject inputs that require them. Do not treat missing support as zero deltas. | Open — backend capability |
| Double-correction guard | LibRaw and rawler reset black metadata after correction. [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-03 §2.4](../RES-03-dnglab.md#24-processing-behavior-and-dng-opcode-boundaries) | darktable uses a float-image/module rule rather than an explicit universal state. [RES-04 §2.7](../RES-04-darktable.md#27-relationship-to-libraw-and-dng) | Represent processing state structurally; metadata absence is not a guard. | Resolved |
| Opcode handling | DNG assigns opcode lists to defined processing stages. [RES-03 §2.4](../RES-03-dnglab.md#24-processing-behavior-and-dng-opcode-boundaries) | The initial `fornax` scope does not yet define gain-map, bad-pixel, or other opcode behavior. [RES-07 §4](../RES-07-rust-dng-tiff.md#4-implications-for-fornax) | Preserve lists; support only an explicitly approved subset and reject required unsupported opcodes. | Open — `SYN-02`/design |

## Universal and backend-specific boundary

Universal concepts:

- stored sample buffer, sample encoding, dimensions, stride, and processing
  state;
- ActiveArea, DefaultCrop, masked areas, and orientation;
- conventional 2×2 RGB CFA layout, plane mapping, and ActiveArea-relative
  phase;
- common, per-position, and repeat-tile black metadata plus white/saturation
  values;
- metadata provenance, camera identity, WB, color matrices, linearization
  metadata, and opcode provenance; and
- structured errors for unsupported layouts or incomplete required metadata.

Backend-specific concepts:

- LibRaw's packed `filters`, `cblack` layout, C structures, and mutating
  `raw2image`/`subtract_black`/`dcraw_process` state;
- rawler's `RawImage`, `BlackLevel`, `Rect`, opcode types, scaling method, and
  panic preconditions;
- rawloader's string errors and fixed four-level metadata; and
- decoder-specific camera catalogs, tag identifiers, and vendor defaults.

These remain behind `RawDecoderBackend` adapters, consistent with the merged
[RES-09 recommendation](../RES-09-rust-ecosystem.md#4-implications-for-fornax).

## Open questions

- Fractional `DefaultCropOrigin`/`DefaultCropSize` require an exact rational
  representation or a documented rejection; `DefaultScale` remains unresolved.
- The current LibRaw wrapper does not use `raw_pitch`, expose black/white/DNG
  crop metadata, or safely handle every raw pointer variant. These are adapter
  implementation gates, not reasons to redefine sample semantics.
- General spatial black tiles and row/column black deltas have incomplete
  backend support and require an exact-origin implementation and fixtures.
- The initial accepted DNG opcode subset and linearization-table coverage are
  not established.
- Some LibRaw vendor decoders may apply format-specific transformations before
  `raw_image`; no target fixture matrix establishes a uniform stored-code
  meaning across DNG, ARW, NEF, and `CR*`.
- Masked-area-derived black estimation is not a dependable fallback in the
  inspected darktable/rawspeed integration and requires separate approval.
- Vendor camera-catalog overrides versus file-declared metadata need an
  explicit provenance conflict policy and fixture evidence.
- Signed integer samples, mixed per-plane encodings, and all float widths are
  outside the established initial target and require structured rejection.

## Acceptance checklist

- [x] Decision table complete for every scoped item.
- [x] Stored versus black-corrected values unambiguous.
- [x] Bit depth, normalization, and white-level semantics stated.
- [x] Scalar, per-channel, and spatially varying black-level representations
  classified.
- [x] Metadata units, provenance, optionality, and conflicts resolved or left
  explicitly open.
- [x] Coordinate spaces and CFA alignment defined.
- [x] Universal versus backend-specific concepts separated.
- [x] Every conclusion links to a supporting report section.
- [x] LibRaw and dnglab are treated as high-priority implementation references
  while normative DNG semantics remain authoritative for DNG.
