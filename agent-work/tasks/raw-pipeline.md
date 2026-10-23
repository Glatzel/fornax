# RAW Pipeline Task Tracker

> **Authority:** `agent-work/raw-pipeline.md` defines the required scope and
> acceptance standards. This file tracks live status, ownership, dependencies,
> and evidence. It does not replace the specification.
>
> **Initialized:** 2026-10-23. Research has not started. No implementation has
> started. No processing code or public API has been modified.
>
> **Rule:** No `SYN-*` task starts until its prerequisite reports are accepted.
> No `DES-*` task is finalized before `SYN-03` is accepted. No `IMPL-*` task
> becomes `ready` until `DES-007` is accepted.

## Status legend

- `blocked` — cannot start; an upstream task or decision is unresolved.
- `ready` — dependencies are satisfied; no owner currently assigned.
- `in_progress` — an owner is actively working and has recorded the task ID.
- `review` — the deliverable exists and is awaiting acceptance review.
- `done` — every acceptance criterion is met and evidence is linked.

Never mark a task `done` because time passed, because a report file exists, or
because a worker reports success. Mark it `done` only after checking the report
and its evidence against the acceptance criteria.

## Evidence standard

Every research and synthesis task must follow these rules:

1. Cite the upstream repository URL, commit or version, source path, symbol,
   line range, test, specification section, or documentation page for each
   material claim.
2. Label each finding:
   - `[verified]` — observed directly in source, tests, specifications, or
     documentation.
   - `[inference]` — a reasoned conclusion not directly observed.
   - `[recommendation]` — a proposed choice for `fornax`, not an upstream fact.
3. Record licensing constraints for any code considered for reuse.
4. Record contradictions between sources and unresolved questions.
5. Do not rely on README descriptions alone; inspect implementation code when
   behavior cannot be established from documentation.
6. Write only the file owned by the task. Research owners write one report;
   `SYN-01` and `SYN-02` write their own notes; `SYN-03` writes `summary.md`.
7. Do not invent APIs, metadata semantics, or implementation details.

Citation form:

```text
[verified] LibRaw `rawdata.raw_image` is a 16-bit sensor-code buffer.
Source: https://github.com/LibRaw/LibRaw @ <commit>; src/libraw_cxx.cpp:L<nn>;
symbol: LibRaw::unpack().
```

## Task overview

| Wave | ID | Title | Status | Dependencies |
| --- | --- | --- | --- | --- |
| R0 | `RES-01` | Audit the existing `fornax` architecture | `review` | none |
| R0 | `RES-02` | Research LibRaw | `review` | none |
| R0 | `RES-03` | Research dnglab and DNG metadata | `blocked` | none |
| R0 | `RES-04` | Research darktable and RAW preprocessing | `ready` | none |
| R0 | `RES-05` | Research demosaicing algorithms and interfaces | `ready` | none |
| R0 | `RES-06` | Survey the Rust RAW-processing ecosystem | `ready` | none |
| S1 | `SYN-01` | Reconcile metadata and sample semantics | `blocked` | `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06` |
| S2 | `SYN-02` | Reconcile preprocessing order and black-level behavior | `blocked` | `RES-02`, `RES-04`, `SYN-01` |
| S3 | `SYN-03` | Cross-source synthesis and recommendations | `blocked` | `RES-01`–`RES-06`, `SYN-01`, `SYN-02` |
| D | `DES-001` | Universal RAW representation and typed common metadata | `blocked` | `SYN-01`, `SYN-02`, `DES-003` |
| D | `DES-002` | Extensible metadata and provenance | `blocked` | `SYN-01` |
| D | `DES-003` | CFA representation and indexing semantics | `blocked` | `SYN-01` |
| D | `DES-004` | Preprocessing operations and processing state | `blocked` | `SYN-01`, `SYN-02`, `DES-001`, `DES-003` |
| D | `DES-005` | Shared demosaicing interface | `blocked` | `SYN-01`, `SYN-03`, `DES-001`, `DES-003` |
| D | `DES-006` | Crate architecture and dependency boundaries | `blocked` | `SYN-03`, `DES-001`, `DES-004`, `DES-005` |
| GATE | `DES-007` | Design review and acceptance gate | `blocked` | `SYN-03`, `DES-001`–`DES-006` |
| I | `IMPL-*` | Implementation backlog | `blocked` | `DES-007` and accepted interfaces |
| Q | `QA-*` | Integration, regression, and validation | `blocked` | relevant `IMPL-*` |

## Wave R0 — Independent reference reports

All six tasks are `ready` at initialization. They may run concurrently because
each owns a different report. A task must not edit another task's report.

### RES-01 — Audit the existing `fornax` architecture

- **Status:** `review`
- **Dependencies:** none
- **Output report:** `agent-work/research/fornax-architecture.md`
- **Scope:** Audit the workspace, crate boundaries, dependency direction,
  features, build scripts, CI, tests, fixtures, benchmarks, error types, naming
  conventions, and coding standards. Inventory `fornax-core`, `fornax-dalim`,
  `libraw`, `libraw-sys`, `fornax`, `fornax-py`, `dnc`, and `fornax-devtool`.
  Determine what `IDecoder`, `IPostProcessor`, `BayerImage`, `BayerPattern`,
  `IDemosaic`, and `DemosaicLinear` actually guarantee.
- **Evidence requirements:** Cite local file paths, line ranges, symbols, and
  test names. Record Cargo manifests, features, and dependency edges. Do not
  infer public behavior from names alone.
- **Acceptance criteria:**
  1. Every public trait, type, and function relevant to decoding, metadata,
     preprocessing, or demosaicing is listed with path and symbol.
  2. Current limitations are explicit, including the absence of metadata,
     black/white levels, crop, and non-Bayer support.
  3. Reuse candidates are separated from code that must be replaced.
  4. Dependency direction is documented, including `fornax-core`'s use of
     `image` and `libraw`'s coupling to `fornax-core`.
  5. The report contains the six required sections and labels evidence.
- **Parallelization:** Independent. Owns only
  `agent-work/research/fornax-architecture.md`.

### RES-02 — Research LibRaw

- **Status:** `review`
- **Review note:** one acceptance criterion is partial: the installed LibRaw 0.22.2 headers declare `black`/`cblack[4104]` but not the repeat-pattern layout or coordinate alignment. See `agent-work/research/libraw.md` §5.
- **Dependencies:** none
- **Output report:** `agent-work/research/libraw.md`
- **Scope:** Trace LibRaw from file opening through unpacking and raw-image
  exposure. Establish the meaning of `rawdata.raw_image`, `color4_image`,
  dimensions, `filters`, black levels, white levels, saturation, crop, masks,
  and margins. Distinguish original sensor codes from unpacked, linearized, or
  already processed samples. Record the relevant public APIs and the behavior of
  the existing `libraw` wrapper.
- **Evidence requirements:** Cite upstream repository URL, version or commit,
  source path, symbol, line range, and tests or documentation where applicable.
  Inspect implementation code rather than relying on README prose. Record
  licensing constraints for considered reuse.
- **Acceptance criteria:**
  1. Each relevant field is documented with type, unit, coordinate convention,
     sentinel values, and validity conditions.
  2. Black-level repeat-pattern and per-channel semantics are explained,
     including alignment with sensor coordinates.
  3. The difference between `raw_image` and processed output is stated.
  4. A field-to-universal-candidate mapping is provided.
  5. Unverified claims are labeled `[inference]`.
  6. The report contains the six required sections and cites sources.
- **Parallelization:** Independent. Owns only `agent-work/research/libraw.md`.
  Coordinates scope with `RES-06` to avoid duplicate ecosystem coverage.

### RES-03 — Research dnglab and DNG metadata

- **Status:** `blocked`
- **Blocker:** dnglab source and the DNG specification are unavailable in the offline environment; only LibRaw DNG-adjacent declarations could be recorded. Re-run when source/network access is available.
- **Dependencies:** none
- **Output report:** `agent-work/research/dnglab.md`
- **Scope:** Inspect dnglab's DNG parsing, raw-image representation, CFA
  handling, black/white level interpretation, active area, default crop,
  orientation, and decoder output semantics. Compare source behavior with the
  DNG specification where the source is ambiguous.
- **Evidence requirements:** Cite dnglab repository URL, version or commit,
  source paths, symbols, tests, and the relevant DNG specification sections.
  Record tag names, types, defaults, units, and coordinate conventions.
- **Acceptance criteria:**
  1. DNG tags relevant to sample interpretation and preprocessing are mapped to
     source symbols, with units, defaults, and coordinate conventions.
  2. `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `CFAPattern`,
     `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`, `Orientation`, and
     related tags are covered or explicitly excluded with a reason.
  3. Differences between DNG and vendor RAW metadata are identified.
  4. Licensing constraints are recorded.
  5. The report contains the six required sections and cites sources.
- **Parallelization:** Independent. Owns only `agent-work/research/dnglab.md`.
  Coordinates scope with `RES-06` to avoid duplicate ecosystem coverage.

### RES-04 — Research darktable and RAW preprocessing

- **Status:** `ready`
- **Dependencies:** none
- **Output report:** `agent-work/research/darktable.md`
- **Scope:** Inspect darktable's actual RAW preprocessing pipeline, including
  rawprepare, temperature, demosaic, and related modules. Determine the order of
  sensor-domain operations, how black and white levels and masks are
  represented, and where demosaicing sits. Verify the exact project and
  repository intended by any reference to "darkroom".
- **Evidence requirements:** Cite darktable repository URL, version or commit,
  module source paths, symbols, and tests. Describe formulas and units. Use
  implementation code for behavior that documentation does not establish.
- **Acceptance criteria:**
  1. The pre-demosaic pipeline order is documented with source paths and
     symbols.
  2. Black-level subtraction, clipping, normalization, and masked-pixel handling
     are described numerically.
  3. The boundary between sensor-domain preprocessing and later image
     processing is explicit.
  4. White balance, color matrices, tone mapping, denoising, and display
     rendering are named as out of scope.
  5. Contradictions with LibRaw or DNG semantics are recorded for `SYN-01` and
     `SYN-02`.
  6. The report contains the six required sections and cites sources.
- **Parallelization:** Independent. Owns only
  `agent-work/research/darktable.md`.

### RES-05 — Research demosaicing algorithms and interfaces

- **Status:** `ready`
- **Dependencies:** none
- **Output report:** `agent-work/research/demosaicing.md`
- **Scope:** Survey bilinear, Malvar–He–Cutler, and representative edge-aware or
  higher-quality algorithms. Record input requirements, CFA assumptions, border
  handling, output semantics, numeric range, metadata needs, and complexity.
  Inspect the existing `fornax-dalim` implementation and benchmark.
- **Evidence requirements:** Cite papers, reference implementations, source
  paths, and tests. Distinguish algorithm properties from implementation
  choices. Record the exact version or commit inspected.
- **Acceptance criteria:**
  1. Candidate algorithms are compared on input assumptions, output semantics,
     border behavior, complexity, and testability.
  2. A recommended initial set is justified; no algorithm is included merely
     because it is well known.
  3. The minimum shared input contract required by all candidates is stated
     without committing to a final public API.
  4. Existing `fornax-dalim` behavior and tests are documented.
  5. The report contains the six required sections and cites sources.
- **Parallelization:** Independent. Owns only
  `agent-work/research/demosaicing.md`. Algorithm implementations remain
  `blocked` until design review.

### RES-06 — Survey the Rust RAW-processing ecosystem

- **Status:** `ready`
- **Dependencies:** none
- **Output report:** `agent-work/research/rust-ecosystem.md`
- **Scope:** Identify relevant Rust RAW decoders, DNG/TIFF libraries, demosaicing
  implementations, reusable components, API designs, dependency choices, and
  licenses. Include rawloader, rawler, image-rs decoders, and other maintained
  projects where relevant. Do not re-document LibRaw or dnglab internals.
- **Evidence requirements:** Cite repository URL, version or commit, source
  paths, symbols, tests, and license files. Record maintenance status and API
  shape. Distinguish reusable code from reusable ideas.
- **Acceptance criteria:**
  1. Each surveyed project has a source-verified summary of its sample
     representation, metadata model, CFA handling, and preprocessing.
  2. Reusable ideas and reusable code are distinguished, with license
     constraints recorded.
  3. Unmaintained or unsuitable projects are named with a reason.
  4. Inferred findings are labeled `[inference]`.
  5. Coverage does not duplicate `RES-02` or `RES-03`.
  6. The report contains the six required sections and cites sources.
- **Parallelization:** Independent. Owns only
  `agent-work/research/rust-ecosystem.md`. Coordinates scope with `RES-02` and
  `RES-03`.

## Wave S — Synthesis

Synthesis cannot start until its prerequisite reports are accepted. Each
synthesis task must distinguish verified facts from interpretation and
recommendation, and must not silently resolve disagreements.

### SYN-01 — Reconcile metadata and sample semantics

- **Status:** `blocked`
- **Dependencies:** `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06`
- **Output:** `agent-work/research/notes/SYN-01.md`
- **Scope:** Reconcile stored versus black-corrected sample values; numeric
  ranges, bit depth, normalization, and white-level semantics; black-level
  representations as metadata, including scalar, per-channel, and spatially
  varying patterns; metadata units, provenance, optionality, and conflicts;
  sensor coordinates, active-area coordinates, cropped-image coordinates, and
  CFA alignment; and which concepts can be universal versus backend-specific.
- **Acceptance criteria:**
  1. A decision table states the chosen meaning, unit, coordinate convention,
     valid range, supporting report sections, authority rule when sources
     disagree, and remaining uncertainty for each item.
  2. Stored versus black-corrected values are unambiguous.
  3. Bit depth, normalization, and white-level semantics are stated.
  4. Scalar, per-channel, and spatially varying black-level representations are
     compared and classified.
  5. Metadata units, provenance, optionality, and conflicts are resolved or
     explicitly left open.
  6. Coordinate spaces and CFA alignment are defined precisely.
  7. Universal versus backend-specific concepts are separated.
  8. Every conclusion links to the supporting report section.
- **Parallelization:** Starts after its five research dependencies are in
  `review` or `done`. Owns only `agent-work/research/notes/SYN-01.md`.

### SYN-02 — Reconcile preprocessing order and black-level behavior

- **Status:** `blocked`
- **Dependencies:** `RES-02`, `RES-04`, `SYN-01`
- **Output:** `agent-work/research/notes/SYN-02.md`
- **Scope:** Reconcile the order and semantics of sensor-domain preprocessing;
  black-level selection, subtraction, clipping, underflow, saturation,
  normalization, and rounding; pattern alignment with crop, margins, and CFA
  phase; prevention of double application; the precise boundary before
  demosaicing; and mandatory, optional, format-dependent, and unsupported
  operations.
- **Acceptance criteria:**
  1. An operation table lists inputs, outputs, formulas, error behavior,
     ordering constraints, and synthetic-test strategy.
  2. Black-level representation and CFA-aware selection are defined.
  3. Subtraction, clipping, underflow, saturation, normalization, and rounding
     are specified numerically.
  4. The boundary before demosaicing is explicit, and later operations that
     must remain unapplied are named.
  5. Missing, invalid, and inconsistent metadata behavior is specified.
  6. Processing-state semantics prevent accidental double correction.
  7. Every decision links to the supporting report or `SYN-01` item.
- **Parallelization:** Starts after `RES-02`, `RES-04`, and `SYN-01` are
  accepted. Owns only `agent-work/research/notes/SYN-02.md`.

### SYN-03 — Cross-source synthesis and recommendations

- **Status:** `blocked`
- **Dependencies:** `RES-01`–`RES-06`, `SYN-01`, `SYN-02`
- **Output:** `agent-work/research/summary.md`
- **Scope:** Compare findings across all six reports and both synthesis notes;
  document agreements, disagreements, uncertainties, and remaining questions;
  make evidence-backed recommendations for the `fornax` architecture.
- **Acceptance criteria:**
  1. Every reference report is represented and linked.
  2. Agreements and disagreements are stated explicitly, with the conflicting
     evidence and a recommended resolution or an explicit deferral.
  3. Recommendations are labeled and traceable to report sections.
  4. Remaining questions and assumptions are listed with impact.
  5. The summary synthesizes rather than concatenates the reports; it does not
     introduce unreferenced claims.
  6. The summary is reviewed before any design task is finalized.
- **Parallelization:** Starts only after all research reports and `SYN-01` /
  `SYN-02` are accepted. Owns only `agent-work/research/summary.md`; no other
  task edits that file.

## Report ownership

| Report | Owning task | Status | Prerequisites | Acceptance focus |
| --- | --- | --- | --- | --- |
| `fornax-architecture.md` | `RES-01` | `review` | none | Architecture, crate boundaries, reusable code, gaps |
| `libraw.md` | `RES-02` | `review` | none | Decoding, sample semantics, CFA, levels, metadata, APIs |
| `dnglab.md` | `RES-03` | `blocked` | none | DNG tags, active area, black patterns, white levels, coordinates |
| `darktable.md` | `RES-04` | `ready` | none | Preprocessing order, black-level behavior, processing boundary |
| `demosaicing.md` | `RES-05` | `ready` | none | Algorithms, input requirements, CFA, output semantics, interface |
| `rust-ecosystem.md` | `RES-06` | `ready` | none | Rust implementations, components, APIs, licenses |
| `summary.md` | `SYN-03` | `blocked` | all `RES-*`, `SYN-01`, `SYN-02` | Cross-source recommendations, disagreements, uncertainties |

Every research task maps to exactly one report, and every report has one owning
task and an explicit acceptance focus. `summary.md` is a synthesis deliverable,
not a seventh research report.

## Wave D — Design

Design tasks produce the concrete API and data-model proposal in
`agent-work/design/raw-pipeline.md`. They must cite the accepted `RES-*` or `SYN-*` task
IDs that justify each decision.

### DES-001 — Universal RAW representation and typed common metadata

- **Status:** `blocked`
- **Dependencies:** `SYN-01`, `SYN-02`, `DES-003`
- **Scope:** Define the canonical decoder output: sample buffer, dimensions,
  stride, type, CFA, black/white levels, active area, crop, provenance, and
  typed metadata. Separate immutable descriptive metadata from mutable
  processing state.
- **Acceptance criteria:** Concrete Rust types and field semantics; units and
  invariants; validation rules; handling of unsupported CFA and non-mosaic data;
  stage invariants; representative code in the design report.
- **Deliverable:** Design report section, citing `SYN-01` and `SYN-02`.
- **Parallelization:** Starts only after its synthesis dependencies are accepted.
  `DES-002` and `DES-003` may run concurrently with it where they do not edit
  the same section.

### DES-002 — Extensible metadata and provenance

- **Status:** `blocked`
- **Dependencies:** `SYN-01`
- **Scope:** Define the key-value metadata model, namespacing, value types,
  units, provenance, conflict resolution, and unknown-field preservation.
- **Acceptance criteria:** Concrete types and keys; authority rules; no generic
  map substituting for typed algorithm-critical fields; provenance retention;
  conversion rules from source metadata.
- **Deliverable:** Design report metadata section, citing `SYN-01`.
- **Parallelization:** May run concurrently with `DES-001` after `SYN-01`.

### DES-003 — CFA representation and indexing semantics

- **Status:** `blocked`
- **Dependencies:** `SYN-01`
- **Scope:** Define CFA pattern types, channel interpretation, origin/phase,
  indexing helpers, and validation. Preserve support for the existing four Bayer
  patterns while allowing explicit rejection of unsupported layouts.
- **Acceptance criteria:** Concrete types and methods; coordinate convention;
  index formulas; bounds and validation behavior; tests for all supported
  patterns; no hidden assumption of a 2×2 Bayer tile in shared APIs.
- **Deliverable:** Design report CFA section, citing `SYN-01`.
- **Parallelization:** Independent of `DES-001` once `SYN-01` is accepted.

### DES-004 — Preprocessing operations and processing state

- **Status:** `blocked`
- **Dependencies:** `SYN-01`, `SYN-02`, `DES-001`, `DES-003`
- **Scope:** Define preprocessing operations, their order, input/output sample
  semantics, metadata requirements, error behavior, allocation policy, and the
  processing-state model that prevents double application.
- **Acceptance criteria:** Concrete operation types and signatures; black-level
  formulas; clipping/normalization rules; missing-metadata behavior; synthetic
  test plan; explicit exclusion of white balance and color matrices.
- **Deliverable:** Design report preprocessing section, citing `SYN-01` and
  `SYN-02`.
- **Parallelization:** After `DES-001` and `DES-003`; blocks `IMPL-PRE-*`.

### DES-005 — Shared demosaicing interface

- **Status:** `blocked`
- **Dependencies:** `SYN-01`, `SYN-03`, `DES-001`, `DES-003`
- **Scope:** Define the stable demosaicing input/output interface, supported
  patterns, border behavior, output semantics, options, and errors.
- **Acceptance criteria:** Concrete trait and types; object-safe or generic
  strategy justified; independent algorithms can be added without changing
  shared types; output metadata policy; test contract.
- **Deliverable:** Design report demosaicing section, citing `SYN-01` and
  `SYN-03`.
- **Parallelization:** After `DES-001` and `DES-003`; unblocks concurrent
  `IMPL-DEM-*` tasks.

### DES-006 — Crate architecture and dependency boundaries

- **Status:** `blocked`
- **Dependencies:** `SYN-03`, `DES-001`, `DES-004`, `DES-005`
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
- **Dependencies:** `SYN-03`, `DES-001`, `DES-002`, `DES-003`, `DES-004`,
  `DES-005`, `DES-006`
- **Scope:** Review the complete design for consistency with accepted synthesis
  decisions, resolve critical open questions, and accept or reject the API.
- **Acceptance criteria:** Every `SYN-*` decision is reflected; unresolved
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
| `IMPL-DEM-003` | Selected edge-aware or higher-quality algorithm | `blocked` | `DES-005`, `DES-007`, `RES-05` | Algorithm justified; numerical and visual checks distinguished; benchmark | Concurrent with `IMPL-DEM-001/002` |
| `IMPL-INT-001` | End-to-end RAW-to-demosaiced pipeline | `blocked` | `IMPL-DEC-004`, `IMPL-PRE-002`, one `IMPL-DEM-*` | Real RAW fixture runs end to end; metadata preserved; no silent post-processing | After decoder, preprocessing, and one demosaicer |
| `QA-001` | Metadata preservation and stage-transition tests | `blocked` | `IMPL-INT-001` | Invariants checked across decode, preprocess, demosaic | After integration |
| `QA-002` | Numerical regression tests | `blocked` | `IMPL-INT-001` | Deterministic fixtures; tolerances explained | After integration |
| `QA-003` | Error-path and unsupported-format tests | `blocked` | `IMPL-INT-001` | Structured errors for invalid layout, missing metadata, unsupported CFA | After integration |
| `QA-004` | Performance baselines and benchmarks | `blocked` | `IMPL-INT-001` | Representative sizes; allocations and copies recorded | After integration |
| `QA-005` | Documentation and existing-API migration notes | `blocked` | `DES-007`, `IMPL-INT-001` | Public behavior documented; breaking changes listed | After design and integration |
| `QA-006` | Workspace-wide CI, lint, and formatting validation | `blocked` | all `IMPL-*` and `QA-*` | Actual command output recorded; failures reported | Final gate |
## Parallel execution plan

**Immediately ready:** `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-05`,
`RES-06`. Each owns a separate report, so all six can proceed concurrently.

**S1:** `SYN-01` after `RES-01`, `RES-02`, `RES-03`, `RES-04`, and `RES-06`
are accepted.

**S2:** `SYN-02` after `SYN-01` and the relevant `RES-02` / `RES-04` findings
are accepted.

**S3:** `SYN-03` after every research report and both synthesis notes are
accepted; it produces `summary.md`.

**Design:** `DES-002` and `DES-003` start after `SYN-01`; `DES-001` also needs
`SYN-02` and `DES-003`; `DES-004` and `DES-005` follow; `DES-006` follows those.
`DES-007` is the gate and requires `SYN-03`.

**Implementation:** `IMPL-DEM-001`, `IMPL-DEM-002`, and `IMPL-DEM-003` can run
concurrently after `DES-005`. `IMPL-PRE-001` can run against synthetic buffers
in parallel with the decoder track after `DES-004`. `IMPL-DEC-002` and
`IMPL-DEC-003` may overlap only with explicit file ownership, because both touch
the decoder adapter.

**Never parallelize:** edits to the same report, `summary.md`, shared types,
workspace manifests, or a shared interface that is still under design.

## Dependency graph

Explicit edges (left side must be accepted before the right side starts):

- `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06` -> `SYN-01`
- `RES-02`, `RES-04`, `SYN-01` -> `SYN-02`
- `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-05`, `RES-06`, `SYN-01`, `SYN-02` -> `SYN-03`
- `SYN-01` -> `DES-002`, `DES-003`
- `SYN-01`, `SYN-02`, `DES-003` -> `DES-001`
- `SYN-01`, `SYN-02`, `DES-001`, `DES-003` -> `DES-004`
- `SYN-01`, `SYN-03`, `DES-001`, `DES-003` -> `DES-005`
- `SYN-03`, `DES-001`, `DES-004`, `DES-005` -> `DES-006`
- `SYN-03`, `DES-001` through `DES-006` -> `DES-007` (design review gate)
- `DES-007` -> every `IMPL-*` task
- completed `IMPL-*` tasks -> relevant `QA-*` tasks

The six `RES-*` tasks have no upstream task dependencies and form the only
immediately parallel wave. No `SYN-*` task may start from a single report alone.
No `DES-*` task may be finalized before `SYN-03`. No `IMPL-*` task may start
before `DES-007`.

## Task update log

| Date | Task | Change | Evidence |
| --- | --- | --- | --- |
| 2026-10-23 | — | Tracker initialized. All research tasks `ready`; synthesis, design, and implementation `blocked`. No research or implementation started. | This file and `agent-work/raw-pipeline.md` |
| 2026-10-23 | `RES-01`–`RES-06`, `SYN-01`–`SYN-03` | Research restructured into one report per source. Added report ownership, synthesis notes, and the `summary.md` gate. Reports are placeholders; no findings recorded. | `agent-work/research/README.md` and the report files |
| 2026-10-23 | `RES-01` | Completed repository architecture audit; report written to `agent-work/research/fornax-architecture.md`; status `review`. | Local commit `d06f426`; report evidence table |
| 2026-10-23 | `RES-02` | Completed LibRaw 0.22.2 declaration and wrapper audit; report written to `agent-work/research/libraw.md`; status `review` with the black-level layout criterion partial. | Installed `libraw_types.h`, `libraw.h`, `libraw_const.h`; wrapper sources |
| 2026-10-23 | `RES-03` | Blocked: dnglab source and DNG specification unavailable offline. Recorded LibRaw DNG-adjacent declarations and open questions; report not accepted. | `agent-work/research/dnglab.md`; failed network probe |
