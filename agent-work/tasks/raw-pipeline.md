# RAW Pipeline Task Tracker

> **Authority:** `agent-work/raw-pipeline.md` defines the required scope and
> acceptance standards. This file tracks live status, ownership, dependencies,
> and evidence. It does not replace the specification.
>
> **Initialized:** 2026-10-23. Research has not started. No implementation has
> started. No processing code or public API has been modified.
>
> **Rule:** No `IMPL-*` task becomes `ready` until `DES-007` (design review) is
> accepted. No design proposal is written until the relevant `SYNTH-*` tasks are
> accepted.

## Status legend

- `blocked` — cannot start; an upstream task or decision is unresolved.
- `ready` — dependencies are satisfied; no owner currently assigned.
- `in_progress` — an owner is actively working and has recorded the task ID.
- `review` — the deliverable exists and is awaiting acceptance review.
- `done` — every acceptance criterion is met and evidence is linked.

Never mark a task `done` because time passed, because a document exists, or
because a worker reports success. Mark it `done` only after checking the
deliverable and the evidence.

## Evidence standard

Every research and synthesis task must follow these rules:

1. Cite the upstream repository URL, commit or version, source path, and symbol
   for each material claim. Cite a test or documentation page when that is the
   evidence.
2. Label each finding:
   - `[verified]` — observed directly in source, tests, or documentation.
   - `[assumption]` — inferred but not verified.
   - `[recommendation]` — a proposed choice for `fornax`, not an upstream fact.
3. Record licensing constraints for any code considered for reuse.
4. Record contradictions between sources and unresolved questions.
5. Do not rely on README descriptions alone.
6. Write working notes only to `agent-work/research/notes/<TASK-ID>.md`.
7. Do not edit `agent-work/research/raw-pipeline.md` from a research task. The
   synthesis integrator owns the consolidated report.

Citation form:

```text
[verified] LibRaw `rawdata.raw_image` is a 16-bit sensor-code buffer.
Source: https://github.com/LibRaw/LibRaw @ <commit>; src/libraw_cxx.cpp:L<nn>;
symbol: LibRaw::unpack().
```

## Task overview

| Wave | ID | Title | Status | Dependencies |
| --- | --- | --- | --- | --- |
| R0 | `RSCH-001` | Existing `fornax` architecture and reusable code | `ready` | none |
| R0 | `RSCH-002` | LibRaw decoding, sample semantics, and metadata | `ready` | none |
| R0 | `RSCH-003` | dnglab and DNG metadata interpretation | `ready` | none |
| R0 | `RSCH-004` | darktable preprocessing and black-level handling | `ready` | none |
| R0 | `RSCH-005` | Demosaicing algorithms and shared input/output contracts | `ready` | none |
| R0 | `RSCH-006` | Other relevant Rust RAW libraries and implementations | `ready` | none |
| S1 | `SYNTH-001` | Metadata fields, units, provenance, and authority | `blocked` | `RSCH-002`, `RSCH-003`, `RSCH-004`, `RSCH-006` |
| S1 | `SYNTH-002` | Coordinate systems, crop, margins, and CFA phase | `blocked` | `RSCH-002`, `RSCH-003`, `SYNTH-001` |
| S1 | `SYNTH-004` | Sample ranges, linearization, and decoder output semantics | `blocked` | `RSCH-001`, `RSCH-002`, `RSCH-003`, `RSCH-006` |
| S2 | `SYNTH-003` | Black-level semantics and pre-demosaic operation order | `blocked` | `RSCH-002`, `RSCH-004`, `SYNTH-001`, `SYNTH-002` |
| S2 | `SYNTH-005` | Demosaicing input/output contract requirements | `blocked` | `RSCH-005`, `SYNTH-002`, `SYNTH-004` |
| D | `DES-001` | Universal RAW representation and typed common metadata | `blocked` | `SYNTH-001`–`SYNTH-004`, `DES-003` |
| D | `DES-002` | Extensible metadata and provenance | `blocked` | `SYNTH-001` |
| D | `DES-003` | CFA representation and indexing semantics | `blocked` | `SYNTH-002` |
| D | `DES-004` | Preprocessing operations and processing state | `blocked` | `SYNTH-003`, `SYNTH-004`, `DES-001`, `DES-003` |
| D | `DES-005` | Shared demosaicing interface | `blocked` | `SYNTH-005`, `DES-001`, `DES-003` |
| D | `DES-006` | Crate architecture and dependency boundaries | `blocked` | `SYNTH-004`, `DES-001`, `DES-004`, `DES-005` |
| GATE | `DES-007` | Design review and acceptance gate | `blocked` | `DES-001`–`DES-006` |
| I | `IMPL-*` | Implementation backlog | `blocked` | `DES-007` and accepted interfaces |
| Q | `QA-*` | Integration, regression, and validation | `blocked` | relevant `IMPL-*` |

## Wave R0 — Independent research

All six tasks are `ready` at initialization. They may run concurrently. Each
worker owns one task ID and one notes file. Research workers must not edit the
consolidated report.

### RSCH-001 — Existing `fornax` architecture and reusable code

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Audit the workspace and every crate manifest: dependency direction,
  features, build scripts, CI, tests, fixtures, benchmarks, error types, naming
  conventions, and coding standards. Inventory `fornax-core`, `fornax-dalim`,
  `libraw`, `libraw-sys`, `fornax`, `fornax-py`, `dnc`, and `fornax-devtool`.
  Determine what the current `IDecoder`, `IPostProcessor`, `BayerImage`,
  `BayerPattern`, `IDemosaic`, and `DemosaicLinear` actually guarantee.
- **Acceptance criteria:**
  1. Every public trait, type, and function relevant to decoding, metadata,
     preprocessing, or demosaicing is listed with file path and symbol.
  2. The current `BayerImage`/`IDemosaic` limitations are stated explicitly,
     including the absence of metadata, black level, white level, crop, and
     non-Bayer support.
  3. Reuse candidates are separated from code that must be replaced.
  4. Dependency direction is documented, including where `fornax-core` reaches
     into `image` and where `libraw` couples to `fornax-core`.
  5. The audit distinguishes `[verified]` facts from `[recommendation]` items.
- **Deliverable:** `agent-work/research/notes/RSCH-001.md`, plus a reviewed
  section and reuse matrix in `agent-work/research/raw-pipeline.md`.
- **Parallelization:** Independent; start immediately. Owns no shared files.

### RSCH-002 — LibRaw decoding, sample semantics, and metadata

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Trace LibRaw from file opening through unpacking and raw-image
  exposure. Determine the meaning of `rawdata.raw_image`, `color4_image`,
  dimensions, `filters`, black levels, white levels, saturation, crop, masks,
  and margins. Distinguish original sensor codes from unpacked, linearized, or
  already processed samples. Inspect the existing `libraw` wrapper for what it
  exposes and what it hides.
- **Acceptance criteria:**
  1. Each relevant LibRaw field is documented with type, unit, coordinate
     convention, sentinel values, and validity conditions.
  2. The exact upstream source path and symbol are cited for each field.
  3. Black-level repeat-pattern semantics and per-channel black levels are
     explained, including how the pattern aligns with sensor coordinates.
  4. The difference between `raw_image` and processed output is stated.
  5. Claims that cannot be verified from source are marked `[assumption]`.
  6. A field-to-universal-candidate mapping is provided.
- **Deliverable:** `agent-work/research/notes/RSCH-002.md`, plus a
  source-verified LibRaw section and metadata examples in the consolidated
  report.
- **Parallelization:** Independent; start immediately. Coordinates with
  `RSCH-006` only to avoid duplicate coverage.

### RSCH-003 — dnglab and DNG metadata interpretation

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Inspect dnglab's DNG parsing, `RawImage` representation, CFA
  handling, black/white level interpretation, crop and orientation metadata,
  and decoder output semantics. Compare source behavior with the DNG
  specification where the source is ambiguous. Record licensing.
- **Acceptance criteria:**
  1. DNG tags relevant to sample interpretation and preprocessing are mapped to
     source symbols, with units, defaults, and coordinate conventions.
  2. `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `CFAPattern`,
     `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`, `Orientation`, and
     related tags are covered or explicitly excluded with a reason.
  3. Differences between DNG metadata and vendor RAW metadata are identified.
  4. Licensing constraints for considered reuse are recorded.
  5. Every material claim is cited to source, tests, or the DNG specification.
- **Deliverable:** `agent-work/research/notes/RSCH-003.md`, plus a
  source-verified dnglab/DNG section and tag mapping table in the consolidated
  report.
- **Parallelization:** Independent; start immediately. Coordinates with
  `RSCH-006` only to avoid duplicate coverage.

### RSCH-004 — darktable preprocessing and black-level handling

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Inspect darktable's rawprepare, temperature, demosaic, and related
  pipeline modules. Determine the order of sensor-domain operations, how black
  and white levels and masks are represented, and which operations occur before
  demosaicing. Verify the exact project and repository intended by any reference
  to "darkroom".
- **Acceptance criteria:**
  1. The pre-demosaic pipeline order is documented with source paths and
     symbols.
  2. Black-level subtraction, clipping, normalization, and masked-pixel
     handling are described with formulas and units.
  3. The scope boundary is explicit: white balance, color matrices, tone
     mapping, denoising, and display rendering are named as out of scope.
  4. Any operation proposed for `fornax` is marked `[recommendation]` and
     justified separately from the upstream description.
  5. Contradictions with LibRaw or DNG semantics are recorded for synthesis.
- **Deliverable:** `agent-work/research/notes/RSCH-004.md`, plus a
  source-verified darktable section and a proposed pre-demosaic operation
  sequence in the consolidated report.
- **Parallelization:** Independent; start immediately.

### RSCH-005 — Demosaicing algorithms and shared input/output contracts

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Survey bilinear, Malvar–He–Cutler, and representative edge-aware or
  higher-quality algorithms. Record CFA assumptions, border handling, output
  channel semantics, numeric range, metadata needs, and complexity. Inspect the
  existing `fornax-dalim` `DemosaicLinear` and benchmark.
- **Acceptance criteria:**
  1. Candidate algorithms are compared on input assumptions, output semantics,
     border behavior, complexity, and testability.
  2. A recommended initial set is justified; no algorithm is included merely
     because it is well known.
  3. The minimum shared input contract required by all candidates is stated
     without committing to a final public API.
  4. Existing `fornax-dalim` behavior and its test expectations are documented.
  5. Claims about upstream algorithms are cited to papers, source, or tests.
- **Deliverable:** `agent-work/research/notes/RSCH-005.md`, plus an algorithm
  survey, contract requirements, and ranked recommendation in the consolidated
  report.
- **Parallelization:** Independent; start immediately. Algorithm
  implementations remain `blocked` until `DES-005` is accepted.

### RSCH-006 — Other relevant Rust RAW libraries and implementations

- **Status:** `ready`
- **Dependencies:** none
- **Scope:** Survey additional Rust RAW decoders, DNG/TIFF libraries, and
  demosaicing implementations not covered by `RSCH-002` or `RSCH-003`. Include
  rawloader, rawler, image-rs decoders, and other maintained projects where
  relevant. Record maintenance status and licensing.
- **Acceptance criteria:**
  1. Each surveyed project has a source-verified summary of its sample
     representation, metadata model, CFA handling, and preprocessing.
  2. Reusable ideas and reusable code are distinguished, with license
     constraints recorded.
  3. Projects that are unmaintained or unsuitable are named with a reason.
  4. Findings that are inferred rather than observed are marked `[assumption]`.
  5. Coverage does not duplicate `RSCH-002` or `RSCH-003`.
- **Deliverable:** `agent-work/research/notes/RSCH-006.md`, plus a comparative
  section and a shortlist of ideas worth adopting in the consolidated report.
- **Parallelization:** Independent; start immediately. Coordinate scope with
  `RSCH-002` and `RSCH-003`; do not re-document LibRaw or dnglab internals.

## Wave S1 and S2 — Synthesis

Synthesis tasks reconcile findings before any universal type or public API is
proposed. Each synthesis task must produce a normative decision table for every
item in its scope: chosen meaning, unit, coordinate convention, valid range,
authority rule when sources disagree, evidence label, and remaining open
questions.

### SYNTH-001 — Metadata fields, units, provenance, and authority

- **Status:** `blocked`
- **Dependencies:** `RSCH-002`, `RSCH-003`, `RSCH-004`, `RSCH-006`
- **Scope:** Reconcile metadata names, meanings, units, defaults, optionality,
  provenance, and conflict resolution across LibRaw, dnglab/DNG, darktable, and
  other implementations. Decide which fields are typed common metadata and
  which belong in extensible metadata.
- **Acceptance criteria:**
  1. A single field catalogue states the canonical name, type, unit, optionality,
     default, and source provenance for each common field.
  2. Every field has an authority rule when sources disagree.
  3. Duplicate or contradictory representations are removed or explicitly
     ranked.
  4. Fields that are preserved but not consumed are identified.
  5. Unknown and vendor-specific fields have a preservation strategy.
  6. Open questions are listed with the design task they block.
- **Deliverable:** `agent-work/research/notes/SYNTH-001.md` and a normative
  metadata section in `agent-work/research/raw-pipeline.md`.
- **Parallelization:** May run concurrently with `SYNTH-002` and `SYNTH-004`
  once its research dependencies are accepted.

### SYNTH-002 — Coordinate systems, crop, margins, and CFA phase

- **Status:** `blocked`
- **Dependencies:** `RSCH-002`, `RSCH-003`, `SYNTH-001`
- **Scope:** Reconcile sensor coordinates, buffer coordinates, active area,
  default crop, margins, masked pixels, orientation, and CFA origin/phase.
  Define how coordinates map between decoder output, metadata patterns, and
  preprocessing.
- **Acceptance criteria:**
  1. Each coordinate space has a name, origin, axis direction, and unit.
  2. Transformations between spaces are stated explicitly, including
     half-open ranges and inclusive bounds.
  3. CFA phase is defined relative to an unambiguous origin, not to an
     unspecified image corner.
  4. Crop and margin interactions with black-level patterns are explained.
  5. Orientation handling is specified or explicitly deferred with a reason.
  6. Invariants that validation code must enforce are listed.
- **Deliverable:** `agent-work/research/notes/SYNTH-002.md` and a coordinate
  and CFA-origin contract in the consolidated report.
- **Parallelization:** Starts after `SYNTH-001`; may run concurrently with
  `SYNTH-003` preparation but `SYNTH-003` waits for its acceptance.

### SYNTH-004 — Sample ranges, linearization, and decoder output semantics

- **Status:** `blocked`
- **Dependencies:** `RSCH-001`, `RSCH-002`, `RSCH-003`, `RSCH-006`
- **Scope:** Reconcile what a decoded sample means: sensor code, unpacked value,
  linearized value, or processed value. Define bit depth, valid range, sentinel
  values, white/saturation level, stride, type, and ownership. Decide what the
  universal decoder output must guarantee.
- **Acceptance criteria:**
  1. The sample domain is defined for every supported decoder backend.
  2. The relationship between raw codes, black level, white level, and
     normalized values is stated.
  3. Type, bit depth, stride, padding, and ownership rules are specified.
  4. Sentinel and out-of-range values are enumerated.
  5. The contract is testable with synthetic buffers and real fixtures.
  6. Unsupported encodings and non-mosaic data are addressed.
- **Deliverable:** `agent-work/research/notes/SYNTH-004.md` and a sample-domain
  and decoder contract in the consolidated report.
- **Parallelization:** May run concurrently with `SYNTH-001` and `SYNTH-002`
  once research dependencies are accepted.

### SYNTH-003 — Black-level semantics and pre-demosaic operation order

- **Status:** `blocked`
- **Dependencies:** `RSCH-002`, `RSCH-004`, `SYNTH-001`, `SYNTH-002`
- **Scope:** Reconcile scalar, per-channel, repeat-pattern, and spatially varying
  black levels. Define subtraction, clipping, normalization, rounding, and the
  ordering of pre-demosaic operations. Define how double application is
  prevented.
- **Acceptance criteria:**
  1. Black-level representation and CFA-aware selection are defined.
  2. The coordinate alignment between black-level patterns, crop, and CFA phase
     is explicit.
  3. Subtraction, clipping, underflow, saturation, and normalization are
     specified numerically.
  4. The pre-demosaic operation order is mandatory, optional, or unsupported per
     operation.
  5. Missing, invalid, and inconsistent metadata behavior is specified.
  6. Processing-state semantics prevent accidental double correction.
  7. Each operation has a synthetic-test strategy.
- **Deliverable:** `agent-work/research/notes/SYNTH-003.md` and the
  preprocessing semantics contract in the consolidated report.
- **Parallelization:** Waits for `SYNTH-001` and `SYNTH-002`. Blocks `DES-004`.

### SYNTH-005 — Demosaicing input/output contract requirements

- **Status:** `blocked`
- **Dependencies:** `RSCH-005`, `SYNTH-002`, `SYNTH-004`
- **Scope:** Reconcile algorithm requirements into a single contract: accepted
  CFA layouts, input sample domain, border handling, output dimensions, channel
  order, numeric type, metadata preservation, and error behavior.
- **Acceptance criteria:**
  1. The minimum shared input and output contract is stated.
  2. Supported and unsupported CFA layouts are enumerated.
  3. Border and output-dimension behavior is unambiguous.
  4. Output channel ordering and numeric range are defined.
  5. Metadata preservation expectations are defined.
  6. Algorithm-specific options are separated from the shared contract.
  7. The contract permits independent algorithm implementations.
- **Deliverable:** `agent-work/research/notes/SYNTH-005.md` and the demosaicing
  interface requirements in the consolidated report.
- **Parallelization:** Waits for `SYNTH-002` and `SYNTH-004`. Blocks `DES-005`.

## Wave D — Design

Design tasks produce the concrete API and data-model proposal in
`agent-work/design/raw-pipeline.md`. They must cite the accepted `SYNTH-*` task
IDs that justify each decision.

### DES-001 — Universal RAW representation and typed common metadata

- **Status:** `blocked`
- **Dependencies:** `SYNTH-001`, `SYNTH-002`, `SYNTH-003`, `SYNTH-004`, `DES-003`
- **Scope:** Define the canonical decoder output: sample buffer, dimensions,
  stride, type, CFA, black/white levels, active area, crop, provenance, and
  typed metadata. Separate immutable descriptive metadata from mutable
  processing state.
- **Acceptance criteria:** Concrete Rust types and field semantics; units and
  invariants; validation rules; handling of unsupported CFA and non-mosaic data;
  stage invariants; representative code in the design report.
- **Deliverable:** Design report section, citing `SYNTH-001`–`SYNTH-004`.
- **Parallelization:** Starts only after its synthesis dependencies are accepted.
  `DES-002` and `DES-003` may run concurrently with it where they do not edit
  the same section.

### DES-002 — Extensible metadata and provenance

- **Status:** `blocked`
- **Dependencies:** `SYNTH-001`
- **Scope:** Define the key-value metadata model, namespacing, value types,
  units, provenance, conflict resolution, and unknown-field preservation.
- **Acceptance criteria:** Concrete types and keys; authority rules; no generic
  map substituting for typed algorithm-critical fields; provenance retention;
  conversion rules from source metadata.
- **Deliverable:** Design report metadata section, citing `SYNTH-001`.
- **Parallelization:** May run concurrently with `DES-001` after `SYNTH-001`.

### DES-003 — CFA representation and indexing semantics

- **Status:** `blocked`
- **Dependencies:** `SYNTH-002`
- **Scope:** Define CFA pattern types, channel interpretation, origin/phase,
  indexing helpers, and validation. Preserve support for the existing four Bayer
  patterns while allowing explicit rejection of unsupported layouts.
- **Acceptance criteria:** Concrete types and methods; coordinate convention;
  index formulas; bounds and validation behavior; tests for all supported
  patterns; no hidden assumption of a 2×2 Bayer tile in shared APIs.
- **Deliverable:** Design report CFA section, citing `SYNTH-002`.
- **Parallelization:** Independent of `DES-001` once `SYNTH-002` is accepted.

### DES-004 — Preprocessing operations and processing state

- **Status:** `blocked`
- **Dependencies:** `SYNTH-003`, `SYNTH-004`, `DES-001`, `DES-003`
- **Scope:** Define preprocessing operations, their order, input/output sample
  semantics, metadata requirements, error behavior, allocation policy, and the
  processing-state model that prevents double application.
- **Acceptance criteria:** Concrete operation types and signatures; black-level
  formulas; clipping/normalization rules; missing-metadata behavior; synthetic
  test plan; explicit exclusion of white balance and color matrices.
- **Deliverable:** Design report preprocessing section, citing `SYNTH-003` and
  `SYNTH-004`.
- **Parallelization:** After `DES-001` and `DES-003`; blocks `IMPL-PRE-*`.

### DES-005 — Shared demosaicing interface

- **Status:** `blocked`
- **Dependencies:** `SYNTH-005`, `DES-001`, `DES-003`
- **Scope:** Define the stable demosaicing input/output interface, supported
  patterns, border behavior, output semantics, options, and errors.
- **Acceptance criteria:** Concrete trait and types; object-safe or generic
  strategy justified; independent algorithms can be added without changing
  shared types; output metadata policy; test contract.
- **Deliverable:** Design report demosaicing section, citing `SYNTH-005`.
- **Parallelization:** After `DES-001` and `DES-003`; unblocks concurrent
  `IMPL-DEM-*` tasks.

### DES-006 — Crate architecture and dependency boundaries

- **Status:** `blocked`
- **Dependencies:** `SYNTH-004`, `DES-001`, `DES-004`, `DES-005`
- **Scope:** Evaluate existing crate boundaries and propose retained or new
  crates, allowed dependency direction, features, public versus internal types,
  and error handling.
- **Acceptance criteria:** No new crate without justification; `fornax-core`
  stays independent of LibRaw; decoder adapters are behind interfaces;
  dependency graph is acyclic; feature and error strategy is documented.
- **Deliverable:** Design report crate-architecture section.
- **Parallelization:** After `DES-001`, `DES-004`, and `DES-005`.

### DES-007 — Design review and acceptance gate

- **Status:** `blocked`
- **Dependencies:** `DES-001`, `DES-002`, `DES-003`, `DES-004`, `DES-005`,
  `DES-006`
- **Scope:** Review the complete design for consistency with accepted synthesis
  decisions, resolve critical open questions, and accept or reject the API.
- **Acceptance criteria:** Every `SYNTH-*` decision is reflected; unresolved
  questions are listed with impact; the implementation backlog is consistent
  with the accepted interfaces; no implementation task is `ready` before this
  gate is `done`.
- **Deliverable:** Review summary in `agent-work/design/raw-pipeline.md` and
  tracker updates.
- **Parallelization:** Single gate. Blocks all `IMPL-*` tasks.

## Wave I — Implementation backlog (provisional, blocked)

These tasks are listed so the roadmap shows the dependency shape. Detailed
scope and acceptance criteria are finalized at `DES-007`. All are `blocked`.

| ID | Scope | Status | Dependencies | Acceptance / deliverable | Parallelization |
| --- | --- | --- | --- | --- | --- |
| `IMPL-DEC-001` | Define the decoder interface adapter to the universal representation | `blocked` | `DES-007`, `DES-001`, `DES-006` | Trait and adapter compile; layout validation; unit tests; owned by decoder workstream | Starts the decoder track |
| `IMPL-DEC-002` | Adapt the LibRaw backend to the decoder interface | `blocked` | `IMPL-DEC-001` | Decodes supported fixtures; layout/CFA checked; no LibRaw types leak downstream | Can overlap `IMPL-DEC-003` only with file ownership split |
| `IMPL-DEC-003` | Map source metadata into typed and extensible metadata | `blocked` | `IMPL-DEC-001`, `DES-002`, `DES-003` | Every consumed field has a mapping test; provenance preserved; unknown fields retained | Coordinates with `IMPL-DEC-002`; shared adapter edits are serialized |
| `IMPL-DEC-004` | Decoder validation and format regression tests | `blocked` | `IMPL-DEC-002`, `IMPL-DEC-003` | Real RAW fixtures pass; semantics documented; failures reported | After decoder and metadata mapping |
| `IMPL-PRE-001` | Scalar black-level correction | `blocked` | `DES-007`, `DES-004`, `DES-001` | Synthetic numerical tests; clipping/rounding explicit; double application detected | Can run in parallel with decoder using synthetic buffers |
| `IMPL-PRE-002` | CFA-aware black-level correction | `blocked` | `IMPL-PRE-001`, `DES-003` | Per-channel and pattern selection tested across all supported Bayer layouts | After `IMPL-PRE-001` |
| `IMPL-PRE-003` | Repeat-pattern or spatially varying black levels | `blocked` | `IMPL-PRE-002` | Pattern alignment with crop and CFA phase tested; unsupported cases rejected | Conditional on research/design justification |
| `IMPL-PRE-004` | Other justified sensor-domain operations | `blocked` | `DES-004`, `DES-007` | Only operations justified by synthesis; numerical tests; no hidden white balance | Independent per operation after design |
| `IMPL-DEM-001` | Bilinear demosaicing | `blocked` | `DES-005`, `DES-007` | Matches reference on synthetic mosaics; borders defined; benchmark | Concurrent with `IMPL-DEM-002/003` |
| `IMPL-DEM-002` | Malvar–He–Cutler demosaicing | `blocked` | `DES-005`, `DES-007` | Numerical comparison against reference; borders; benchmark | Concurrent with `IMPL-DEM-001/003` |
| `IMPL-DEM-003` | Selected edge-aware or higher-quality algorithm | `blocked` | `DES-005`, `DES-007`, `RSCH-005` | Algorithm justified; numerical and visual checks distinguished; benchmark | Concurrent with `IMPL-DEM-001/002` |
| `IMPL-INT-001` | End-to-end RAW-to-demosaiced pipeline | `blocked` | `IMPL-DEC-004`, `IMPL-PRE-002`, one `IMPL-DEM-*` | Real RAW fixture runs end to end; metadata preserved; no silent post-processing | After decoder, preprocessing, and one demosaicer |
| `QA-001` | Metadata preservation and stage-transition tests | `blocked` | `IMPL-INT-001` | Invariants checked across decode, preprocess, demosaic | After integration |
| `QA-002` | Numerical regression tests | `blocked` | `IMPL-INT-001` | Deterministic fixtures; tolerances explained | After integration |
| `QA-003` | Error-path and unsupported-format tests | `blocked` | `IMPL-INT-001` | Structured errors for invalid layout, missing metadata, unsupported CFA | After integration |
| `QA-004` | Performance baselines and benchmarks | `blocked` | `IMPL-INT-001` | Representative sizes; allocations and copies recorded | After integration |
| `QA-005` | Documentation and existing-API migration notes | `blocked` | `DES-007`, `IMPL-INT-001` | Public behavior documented; breaking changes listed | After design and integration |
| `QA-006` | Workspace-wide CI, lint, and formatting validation | `blocked` | all `IMPL-*` and `QA-*` | Actual command output recorded; failures reported | Final gate |
## Parallel execution plan

**Immediately ready:** `RSCH-001`, `RSCH-002`, `RSCH-003`, `RSCH-004`,
`RSCH-005`, `RSCH-006`.

**Blocked until research is accepted:** all `SYNTH-*` tasks.

**Concurrent after Wave R0 acceptance:** `SYNTH-001`, `SYNTH-002`, `SYNTH-004`.

**Then:** `SYNTH-003` after `SYNTH-001` and `SYNTH-002`; `SYNTH-005` after
`SYNTH-002` and `SYNTH-004`.

**Design:** `DES-002` and `DES-003` can start once their synthesis inputs are
accepted; `DES-001` also needs `DES-003`. `DES-004` and `DES-005` follow;
`DES-006` follows those; `DES-007` is the gate.

**Implementation:** `IMPL-DEM-001`, `IMPL-DEM-002`, and `IMPL-DEM-003` can run
concurrently after `DES-005`. `IMPL-PRE-001` can run against synthetic buffers
in parallel with the decoder track after `DES-004`. `IMPL-DEC-002` and
`IMPL-DEC-003` may overlap only with explicit file ownership, because both touch
the decoder adapter.

**Never parallelize:** edits to shared types, workspace manifests, the
consolidated research report, or a shared interface that is still under design.

## Dependency graph

Explicit edges (left side must be accepted before the right side starts):

- `RSCH-002`, `RSCH-003`, `RSCH-004`, `RSCH-006` -> `SYNTH-001`
- `RSCH-002`, `RSCH-003`, `SYNTH-001` -> `SYNTH-002`
- `RSCH-001`, `RSCH-002`, `RSCH-003`, `RSCH-006` -> `SYNTH-004`
- `RSCH-002`, `RSCH-004`, `SYNTH-001`, `SYNTH-002` -> `SYNTH-003`
- `RSCH-005`, `SYNTH-002`, `SYNTH-004` -> `SYNTH-005`
- `SYNTH-001` -> `DES-002`
- `SYNTH-002` -> `DES-003`
- `SYNTH-001`, `SYNTH-002`, `SYNTH-003`, `SYNTH-004`, `DES-003` -> `DES-001`
- `SYNTH-003`, `SYNTH-004`, `DES-001`, `DES-003` -> `DES-004`
- `SYNTH-005`, `DES-001`, `DES-003` -> `DES-005`
- `SYNTH-004`, `DES-001`, `DES-004`, `DES-005` -> `DES-006`
- `DES-001` through `DES-006` -> `DES-007` (design review gate)
- `DES-007` -> every `IMPL-*` task
- completed `IMPL-*` tasks -> relevant `QA-*` tasks

The six `RSCH-*` tasks have no upstream task dependencies and form the only
immediately parallel wave. No `DES-*` task may start from a research finding
alone; it must cite an accepted `SYNTH-*` decision. No `IMPL-*` task may start
before `DES-007`.

## Task update log

| Date | Task | Change | Evidence |
| --- | --- | --- | --- |
| 2026-10-23 | — | Tracker initialized. All research tasks `ready`; synthesis, design, and implementation `blocked`. No research or implementation started. | This file and `agent-work/raw-pipeline.md` |
