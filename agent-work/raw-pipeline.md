# Task: Research, Design, and Implement a Rust-Native RAW Processing Pipeline in `fornax`

> **Status:** Initialised. Research tasks are defined and `ready`; design and
> implementation have not started.
>
> **Authority:** This file is the authoritative task specification for the RAW
> processing pipeline. `AGENTS.md` at the repository root contains only concise
> repository instructions and must not duplicate or override this specification.
>
> **Deliverable layout:** The original prompt used `docs/` paths. Those paths are
> mapped to this repository's `agent-work/` directory. The hidden `.agents/`
> directory is mounted read-only in the agent runtime, so `agent-work/` is the
> canonical location for agent materials.
>
> ```text
> agent-work/
> ├── raw-pipeline.md
> ├── research/
> │   ├── README.md
> │   ├── fornax-architecture.md
> │   ├── libraw.md
> │   ├── dnglab.md
> │   ├── darktable.md
> │   ├── demosaicing.md
> │   ├── rust-ecosystem.md
> │   ├── summary.md
> │   └── notes/                   # synthesis working notes
> ├── design/raw-pipeline.md
> └── tasks/raw-pipeline.md
> ```
>
> Each reference report has exactly one owning research task. `summary.md` is
> produced by `SYN-03`; it synthesizes the reports rather than replacing them.
> Completed report content, design content, and tracker updates are future
> deliverables. The report files currently define required structure only. Do
> not fabricate findings during initialization, and do not begin the research
> phase until instructed.

## 1. Project context and objective

`fornax` is an existing Rust RAW image processing repository containing crates related to LibRaw bindings, Rust demosaicing implementations, Python bindings, and related functionality.

Evolve the existing repository toward a Rust-native RAW processing library. Use the existing codebase and relevant upstream RAW processing libraries as references. Preserve useful implementations and compatibility where practical; do not rewrite the repository from scratch without justification.

The target pipeline is:

1. **Decode:** Read a supported RAW file and produce a universal, backend-independent RAW representation with sufficient metadata for subsequent processing.
2. **Preprocess:** Interpret the decoded sensor metadata and perform explicitly defined RAW-domain corrections, especially black-level correction.
3. **Demosaic:** Reconstruct a full-resolution image from the sensor mosaic using interchangeable algorithms.

The pipeline ends at demosaicing. Do not implement downstream white balance application, color correction, color-space conversion, tone mapping, denoising, sharpening, or display rendering.

The primary deliverable is a well-researched and documented design, followed by an implementation whose progress and dependencies are explicitly tracked.

## 2. Phase A — Repository audit and reference research

Before modifying the architecture, inspect the current repository.

Examine:

- Workspace and crate manifests, dependency relationships, features, build scripts, CI, tests, and documentation.
- Existing LibRaw bindings and how they expose RAW samples, dimensions, CFA information, black levels, white levels, camera metadata, and other decoder output.
- Existing Rust demosaicing code, supported CFA patterns, algorithms, and public interfaces.
- Python bindings and compatibility requirements.
- Existing fixtures, benchmarks, error types, naming conventions, and coding standards.

Research relevant upstream projects, including:

- LibRaw.
- dnglab.
- darktable and other relevant RAW processing implementations, verifying the exact project and repository meant by any reference to darkroom.
- Other relevant Rust RAW decoders, DNG/TIFF libraries, and demosaicing implementations.

Inspect source code, documentation, tests, and sample files where available. Do not rely exclusively on README descriptions.

### Required research questions

For each relevant reference implementation, investigate:

1. How is a RAW file parsed, unpacked, and decoded?
2. What does its decoded pixel buffer represent: original sensor codes, unpacked samples, linearized samples, or already processed data?
3. How are image dimensions, stride, sample type, CFA pattern, and CFA origin represented?
4. How are black level, white level, saturation, masked pixels, and spatially varying black levels represented and interpreted?
5. How are active area, crop area, margins, and sensor orientation handled?
6. Which metadata is retained, normalized, derived, or discarded?
7. Which metadata is consumed before demosaicing, and exactly how does it affect the pixel values or CFA interpretation?
8. Which demosaicing algorithms are available, and what are their input assumptions and output semantics?
9. Which parts of the existing `fornax` implementation can be reused?
10. Which behavior is format-specific and which belongs in a universal API?

Distinguish source-verified facts from assumptions and recommendations. Record upstream URLs, source paths, relevant symbols, and licensing constraints for code considered for reuse.

### Research reports and tracked tasks

Research is a first-class, independently tracked phase. Each source has exactly
one report and exactly one owning task; tasks must not share report files. Live
status is maintained in `agent-work/tasks/raw-pipeline.md`; this section defines
the normative scope.

| ID | Report | Focus | Depends on | Parallel |
| --- | --- | --- | --- | --- |
| `RES-01` | `agent-work/research/fornax-architecture.md` | Existing repository, crates, data structures, dependencies, reusable code | none | Yes, wave R0 |
| `RES-02` | `agent-work/research/libraw.md` | LibRaw decoding, sample semantics, CFA, black/white levels, metadata, APIs | none | Yes, wave R0 |
| `RES-03` | `agent-work/research/dnglab.md` | DNG metadata, tags, active area, black-level patterns, white levels, coordinates | none | Yes, wave R0 |
| `RES-04` | `agent-work/research/darktable.md` | RAW preprocessing pipeline, black-level correction, CFA handling, processing boundary | none | Yes, wave R0 |
| `RES-05` | `agent-work/research/demosaicing.md` | Demosaicing algorithms, inputs, CFA handling, output semantics, common interfaces | none | Yes, wave R0 |
| `RES-06` | `agent-work/research/rust-ecosystem.md` | Rust RAW decoders, demosaicing implementations, components, API designs, dependencies, licenses | none | Yes, wave R0 |

Each research task owns exactly one report. A task may reference another report,
but it must not edit it. The six tasks form wave R0 and may run concurrently.

#### RES-01 — Audit the existing `fornax` architecture

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
- **Acceptance criteria:** Every public trait, type, and function relevant to
  decoding, metadata, preprocessing, or demosaicing is listed with path and
  symbol. Current limitations are explicit, including the absence of metadata,
  black/white levels, crop, and non-Bayer support. Reuse candidates are
  separated from code that must be replaced. Dependency direction is documented.
  The report satisfies the required report structure.
- **Parallelization:** Independent. Owns only
  `agent-work/research/fornax-architecture.md`.

#### RES-02 — Research LibRaw

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
- **Acceptance criteria:** Each relevant field is documented with type, unit,
  coordinate convention, sentinel values, and validity conditions. Black-level
  repeat-pattern and per-channel semantics are explained, including alignment
  with sensor coordinates. The difference between `raw_image` and processed
  output is stated. A field-to-universal-candidate mapping is provided. Claims
  that cannot be verified are labeled `[inference]`. The report satisfies the
  required report structure.
- **Parallelization:** Independent. Owns only `agent-work/research/libraw.md`.
  Coordinates scope with `RES-06` to avoid duplicate ecosystem coverage.

#### RES-03 — Research dnglab and DNG metadata

- **Output report:** `agent-work/research/dnglab.md`
- **Scope:** Inspect dnglab's DNG parsing, raw-image representation, CFA
  handling, black/white level interpretation, active area, default crop,
  orientation, and decoder output semantics. Compare source behavior with the
  DNG specification where the source is ambiguous.
- **Evidence requirements:** Cite dnglab repository URL, version or commit,
  source paths, symbols, tests, and the relevant DNG specification sections.
  Record tag names, types, defaults, units, and coordinate conventions.
- **Acceptance criteria:** DNG tags relevant to sample interpretation and
  preprocessing are mapped to source symbols, with units, defaults, and
  coordinate conventions. `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`,
  `CFAPattern`, `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`,
  `Orientation`, and related tags are covered or explicitly excluded with a
  reason. Differences between DNG and vendor RAW metadata are identified.
  Licensing constraints are recorded. The report satisfies the required report
  structure.
- **Parallelization:** Independent. Owns only `agent-work/research/dnglab.md`.
  Coordinates scope with `RES-06` to avoid duplicate ecosystem coverage.

#### RES-04 — Research darktable and RAW preprocessing

- **Output report:** `agent-work/research/darktable.md`
- **Scope:** Inspect darktable's actual RAW preprocessing pipeline, including
  rawprepare, temperature, demosaic, and related modules. Determine the order of
  sensor-domain operations, how black and white levels and masks are
  represented, and where demosaicing sits. Verify the exact project and
  repository intended by any reference to "darkroom".
- **Evidence requirements:** Cite darktable repository URL, version or commit,
  module source paths, symbols, and tests. Describe formulas and units. Use
  implementation code for behavior that documentation does not establish.
- **Acceptance criteria:** The pre-demosaic pipeline order is documented with
  source paths and symbols. Black-level subtraction, clipping, normalization,
  and masked-pixel handling are described numerically. The boundary between
  sensor-domain preprocessing and later image processing is explicit. White
  balance, color matrices, tone mapping, denoising, and display rendering are
  named as out of scope. Contradictions with LibRaw or DNG semantics are
  recorded for synthesis. The report satisfies the required report structure.
- **Parallelization:** Independent. Owns only
  `agent-work/research/darktable.md`.

#### RES-05 — Research demosaicing algorithms and interfaces

- **Output report:** `agent-work/research/demosaicing.md`
- **Scope:** Survey bilinear, Malvar–He–Cutler, and representative edge-aware or
  higher-quality algorithms. Record input requirements, CFA assumptions, border
  handling, output semantics, numeric range, metadata needs, and complexity.
  Inspect the existing `fornax-dalim` implementation and benchmark.
- **Evidence requirements:** Cite papers, reference implementations, source
  paths, and tests. Distinguish algorithm properties from implementation
  choices. Record the exact version or commit of inspected implementations.
- **Acceptance criteria:** Candidate algorithms are compared on input
  assumptions, output semantics, border behavior, complexity, and testability. A
  recommended initial set is justified; no algorithm is included merely because
  it is well known. The minimum shared input contract required by all candidates
  is stated without committing to a final public API. Existing `fornax-dalim`
  behavior and tests are documented. The report satisfies the required report
  structure.
- **Parallelization:** Independent. Owns only
  `agent-work/research/demosaicing.md`. Algorithm implementations remain blocked
  until design review.

#### RES-06 — Survey the Rust RAW-processing ecosystem

- **Output report:** `agent-work/research/rust-ecosystem.md`
- **Scope:** Identify relevant Rust RAW decoders, DNG/TIFF libraries, demosaicing
  implementations, reusable components, API designs, dependency choices, and
  licenses. Include rawloader, rawler, image-rs decoders, and other maintained
  projects where relevant. Do not re-document LibRaw or dnglab internals.
- **Evidence requirements:** Cite repository URL, version or commit, source
  paths, symbols, tests, and license files. Record maintenance status and API
  shape. Distinguish reusable code from reusable ideas.
- **Acceptance criteria:** Each surveyed project has a source-verified summary of
  its sample representation, metadata model, CFA handling, and preprocessing.
  Reusable ideas and reusable code are distinguished, with license constraints
  recorded. Unmaintained or unsuitable projects are named with a reason.
  Findings that are inferred are labeled `[inference]`. The report satisfies the
  required report structure.
- **Parallelization:** Independent. Owns only
  `agent-work/research/rust-ecosystem.md`. Coordinates scope with `RES-02` and
  `RES-03`.

### Required report structure

Every reference report (`fornax-architecture.md`, `libraw.md`, `dnglab.md`,
`darktable.md`, `demosaicing.md`, `rust-ecosystem.md`) must contain:

1. **Scope and questions** — what was investigated and the specific questions.
2. **Findings** — specific technical behavior and design details.
3. **Evidence** — upstream documentation, source files, symbols, line ranges,
   code locations, tests, or specification sections supporting each significant
   claim.
4. **Implications for fornax** — what the findings mean for the API, data model,
   and pipeline.
5. **Open questions and limitations** — unverified assumptions, missing
   evidence, ambiguities, and unresolved issues.
6. **References** — direct links to the relevant upstream sources.

Front matter must record the owning task ID, status, owner, last update, and the
upstream versions or commits inspected. Each report must include the acceptance
criteria of its owning task and a reviewer must check them before the task moves
to `done`.

### Evidence and provenance rules

Each report must distinguish:

- **Verified upstream behavior** — observed directly in source, tests,
  specifications, or documentation. Label `[verified]`.
- **Interpretation or inference** — a reasoned conclusion not directly observed.
  Label `[inference]`.
- **Proposed design recommendation** — a proposed choice for `fornax`, not an
  upstream fact. Label `[recommendation]`.

Every material claim must cite the upstream repository URL, version or commit,
source path, and symbol or line range, or a test, specification section, or
documentation page. Prefer primary sources. Inspect implementation code when
documentation cannot establish behavior. Do not invent APIs, metadata semantics,
or implementation details. Record disagreements between sources and missing
evidence instead of smoothing them over. Keep each report within its own scope
and reference other reports by path and section rather than copying them.

### Synthesis tasks

Synthesis is separate from reference research and cannot start until its
prerequisite reports are accepted. It must not silently reconcile disagreements.

| ID | Output | Reconciles | Depends on |
| --- | --- | --- | --- |
| `SYN-01` | `agent-work/research/notes/SYN-01.md` | Metadata and sample semantics | `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06` |
| `SYN-02` | `agent-work/research/notes/SYN-02.md` | Preprocessing order and black-level behavior | `RES-02`, `RES-04`, `SYN-01` |
| `SYN-03` | `agent-work/research/summary.md` | Cross-source synthesis and recommendations | `RES-01`–`RES-06`, `SYN-01`, `SYN-02` |

`SYN-01` must specifically examine:

- Stored versus black-corrected sample values.
- Numeric ranges, bit depth, normalization, and white-level semantics.
- Black-level representations as metadata: scalar, per-channel, and spatially
  varying patterns.
- Metadata units, provenance, optionality, and conflicts between sources.
- Sensor coordinates, active-area coordinates, cropped-image coordinates, and
  CFA alignment.
- Which concepts can be represented by universal types and which must remain
  backend-specific.

It must produce a decision table stating, per item, the chosen meaning, unit,
coordinate convention, valid range, supporting report sections, authority rule
when sources disagree, and remaining uncertainty.

`SYN-02` must specifically examine:

- The order and semantics of sensor-domain preprocessing.
- Black-level selection, subtraction, clipping, underflow, saturation,
  normalization, and rounding.
- Pattern alignment with crop, margins, and CFA phase.
- How to prevent applying the same correction twice.
- The precise boundary before demosaicing, and which later operations must
  remain unapplied.
- Mandatory, optional, format-dependent, and unsupported operations.

It must produce an operation table with inputs, outputs, formulas, error
behavior, ordering constraints, and synthetic-test strategy.

`SYN-03` must:

- Compare findings across all six reference reports.
- Document agreements, disagreements, uncertainties, and remaining questions.
- Make evidence-backed recommendations for the `fornax` architecture.
- Link each conclusion to the supporting report and section. It must not
  concatenate the reports or introduce unreferenced claims.
- State the assumptions behind every recommendation.
- Be reviewed before any design task is finalized.

### Research completion gate

- No synthesis task starts until every listed dependency report is in `review`
  or `done`.
- No design task is finalized before `SYN-03` is accepted. Design decisions must
  cite the supporting `RES-*` or `SYN-*` task IDs, or explicitly state their
  assumptions.
- No implementation task starts before the `DES-007` design review gate.
- The research phase is complete only when `RES-01`–`RES-06` and `SYN-01`–
  `SYN-03` are `done` with satisfied acceptance criteria.
- A report file existing is not evidence of completion.

### Research deliverables

```text
agent-work/research/
├── README.md
├── fornax-architecture.md
├── libraw.md
├── dnglab.md
├── darktable.md
├── demosaicing.md
├── rust-ecosystem.md
├── summary.md
└── notes/
    ├── SYN-01.md
    └── SYN-02.md
```

`README.md` is the index of reports, owning tasks, status, dependencies, and
acceptance criteria. `summary.md` is produced only by `SYN-03`. Research owners
write only their own report; synthesis working notes are owned by `SYN-01` and
`SYN-02`.

Do not begin a broad implementation before the research reports and synthesis
are accepted.

## 3. Phase B — API and data structure design

Create `agent-work/design/raw-pipeline.md` as a detailed design proposal.

Design work is tracked as `DES-001` through `DES-006`, followed by the
`DES-007` review gate. A design task may start only when its listed `SYN-*`
dependencies are accepted, and its proposal must cite the `RES-*` or `SYN-*`
task IDs that justify each decision. No public type or function is finalized
before `SYN-03` is accepted. Do not introduce a public type or function whose
metadata, coordinate, sample-domain, or preprocessing semantics are still
unresolved by synthesis.

The design must be consistent with the research report and the existing repository. It should describe actual proposed Rust types, fields, invariants, function signatures, data flow, and dependency boundaries rather than merely listing concepts.

### 3.1 Universal RAW representation

Define the canonical representation produced by the decoder.

Evaluate the following information and specify the exact type, units, semantics, and validation rules for each applicable field:

- Sensor sample buffer, dimensions, stride, sample format, and ownership.
- CFA pattern, channel interpretation, and spatial phase/origin.
- Black level and its repeat pattern or per-channel/per-position variation.
- White level, saturation information, and any relevant linearization semantics.
- Active image area, default crop, masked areas, and margins.
- Sensor dimensions versus decoded buffer dimensions.
- Camera make, model, and sensor identifiers where available.
- White-balance coefficients and as-shot metadata.
- Camera color matrices and calibration metadata.
- Exposure and other relevant capture metadata.
- Original file metadata, decoder provenance, and source-format information.

Do not assume that all RAW files contain a conventional Bayer mosaic.

Explicitly define how unsupported CFA layouts, monochrome sensors, unusual sample encodings, and non-mosaic RAW data are represented or rejected.

Separate immutable descriptive metadata from mutable processing state. Define which invariants must hold before and after each pipeline stage.

### 3.2 Metadata architecture

Use two complementary mechanisms:

**Typed common metadata**

Provide named, statically typed fields for metadata required to interpret sensor samples or commonly consumed by preprocessing and demosaicing.

**Extensible metadata**

Provide a structured key-value representation for less common, vendor-specific, and format-specific metadata, preserving original values and provenance where feasible.

Specify:

- Metadata key naming and namespacing.
- Value types, units, and coordinate systems.
- Optionality and defaults.
- Source provenance and confidence where relevant.
- Conflict resolution when multiple metadata sources disagree.
- Preservation of unknown fields.
- Conversion rules between source-format metadata and universal fields.

Do not use a generic hash map as a substitute for defining the typed fields needed by algorithms. Avoid duplicate, contradictory representations of the same metadata without an explicit authority rule.

### 3.3 Metadata-driven preprocessing before demosaicing

This is a central part of the design, not an implementation detail to defer.

Document a precise processing sequence, including which operations are mandatory, optional, format-dependent, or unsupported.

For each operation, state:

- Required metadata.
- Input and output sample semantics.
- Numerical formula or algorithm.
- Buffer mutation and allocation behavior.
- Handling of missing, invalid, or inconsistent metadata.
- Clipping, underflow, saturation, and rounding behavior.
- Whether the operation must precede demosaicing.
- How to test it using synthetic samples.

At minimum, analyze black-level correction:

- Scalar versus per-channel or spatially varying black levels.
- CFA-aware selection of the appropriate black level.
- Black-level repeat patterns and sensor-coordinate alignment.
- Cropping and margins that affect coordinate mapping.
- Subtraction in the source sample domain.
- The distinction between subtracting the black level, clipping negative corrected values, and normalizing by the available signal range.

Also research whether additional operations belong before demosaicing, such as linearization, handling of masked sensor pixels, or defective-pixel correction. Include them only when their semantics and metadata requirements are justified.

Do not automatically apply white balance or camera color matrices merely because the metadata is present.

Define how preprocessing is represented in the API so that a caller cannot accidentally apply the same correction twice without detection or an explicitly documented opt-in.

### 3.4 Demosaicing API

Design a shared, stable input/output interface for interchangeable demosaicing algorithms.

Specify:

- Required preprocessed input representation.
- CFA patterns and sample types supported by the interface.
- Border handling and output dimensions.
- Output channel ordering and numeric representation.
- Treatment of unsupported patterns.
- Whether metadata is preserved in the output.
- Algorithm-specific options and defaults.
- Error handling and allocation behavior.

Design the interface so that algorithm implementations can be developed independently and concurrently.

Research and recommend an initial set of algorithms. Treat algorithm names such as nearest-neighbor, bilinear, Malvar–He–Cutler, and edge-aware methods as candidates to evaluate, not an unconditional requirement to implement every one.

### 3.5 Crate architecture

Evaluate existing crate boundaries before proposing new crates.

For each proposed or retained crate, document:

- Responsibility and public API.
- Dependencies and allowed dependency direction.
- Public versus internal types.
- Features and optional dependencies.
- Error handling.
- Whether it can be implemented and tested independently.

Keep the core data model independent of LibRaw-specific types. Existing bindings may remain as one decoder backend or as a reference for compatibility, but downstream processing must not depend on them.

Include representative Rust type definitions and function signatures in the design report.

## 4. Phase C — Tracked backlog and task tracking

Create `agent-work/tasks/raw-pipeline.md` as the living tracker for every
phase, including research and synthesis. Research, synthesis, design, and
implementation tasks all use the same tracking rules.

Every tracked task must have:

- A stable task ID.
- A concise title and scope.
- Status: `blocked`, `ready`, `in_progress`, `review`, or `done`.
- Dependencies expressed as task IDs.
- Acceptance criteria.
- Deliverable, including the file where the result is recorded.
- Relevant crates and files, or research sources.
- Parallelization notes.
- Test, validation, or review requirements.

Use Markdown checklists or tables that are easy to update in code review. Keep
the tracker synchronized with actual progress; never mark work complete without
evidence. A task with unresolved acceptance criteria stays in `in_progress` or
`review`, not `done`.

### Task ID scheme

| Prefix | Phase | Typical status at initialization |
| --- | --- | --- |
| `RES-` | Independent reference research | `ready` |
| `SYN-` | Cross-source synthesis and reconciliation | `blocked` |
| `DES-` | API, data model, and architecture design | `blocked` |
| `IMPL-` | Implementation | `blocked` |
| `QA-` | Integration, validation, and performance | `blocked` |

### Initial work breakdown

Create and refine tasks covering at least the following workstreams.

**A. Research, synthesis, and shared contracts**

- `RES-01` through `RES-06`: the six independent report-owning research tasks
  defined in Section 2. All are `ready` at initialization and may run
  concurrently. Each report has exactly one owner.
- `SYN-01` through `SYN-03`: reconcile metadata and sample semantics,
  preprocessing order and black-level behavior, and cross-source
  recommendations. Each is `blocked` on its listed reports; `SYN-03` produces
  `agent-work/research/summary.md`.
- Review and acceptance of the API/data model design (`DES-007`).
- Universal RAW representation and common metadata (`DES-001`).
- Extensible metadata and provenance (`DES-002`).
- CFA representation and indexing semantics (`DES-003`).
- Preprocessing semantics and processing state (`DES-004`).
- Shared demosaicing interface (`DES-005`).
- Crate architecture and dependency boundaries (`DES-006`).
- Shared error types and validation.
- Test fixture conventions and reference-comparison methodology.

**B. Decoding**

- Define the decoder interface.
- Implement or adapt the initial RAW decoder backend.
- Map source metadata into the universal representation.
- Validate buffer layout, sample semantics, and CFA alignment.
- Add decoder integration tests and format-specific regression tests.

**C. Preprocessing**

- Define preprocessing operations and processing-state semantics.
- Implement scalar black-level correction.
- Implement CFA-aware black-level correction.
- Handle repeat-pattern or spatially varying black levels when supported.
- Investigate and implement other justified sensor-domain operations as separate tasks.
- Add synthetic numerical tests and reference comparisons.

Split tasks further if an operation requires different algorithms, formats, or independent acceptance criteria.

**D. Demosaicing**

Create separate tasks for each selected algorithm, for example:

- Demosaic algorithm A: bilinear baseline.
- Demosaic algorithm B: Malvar–He–Cutler.
- Demosaic algorithm C: selected edge-aware or higher-quality algorithm.

These are example work items. Finalize the list based on research.

Each algorithm must have independent implementation and test criteria. Multiple algorithms should be implementable concurrently after the shared interface and CFA semantics are stable.

**E. Integration and quality**

- End-to-end RAW-to-demosaiced-image pipeline.
- Metadata preservation and stage-transition tests.
- Numerical regression tests.
- Error-path and unsupported-format tests.
- Performance baselines and benchmarks.
- Documentation and migration of existing APIs.
- Workspace-wide CI and lint validation.

### Dependency and parallelization rules

Represent dependencies explicitly. The dependency graph is:

`RES-01..06 → SYN-01 → SYN-02 → SYN-03 → DES-001..007 → IMPL-* → QA-*`

Execution waves:

- **Wave R0 — independent research.** `RES-01` through `RES-06` are `ready`
  simultaneously. Each worker owns one task ID and exactly one report file.
- **Wave S1 — metadata and sample synthesis.** `SYN-01` starts after `RES-01`,
  `RES-02`, `RES-03`, `RES-04`, and `RES-06` are in `review` or `done`.
- **Wave S2 — preprocessing synthesis.** `SYN-02` starts after `RES-02`,
  `RES-04`, and `SYN-01` are accepted.
- **Wave S3 — cross-source synthesis.** `SYN-03` starts after all six reports and
  both synthesis notes are accepted, and produces `summary.md`.
- **Wave D — design.** `DES-001` through `DES-006` start only after their listed
  `SYN-*` dependencies are accepted. `DES-007` also requires `SYN-03`.
- **Wave I — implementation.** No `IMPL-*` task is `ready` until `DES-007` is
  accepted and the affected interfaces are stable.

After the common data model, CFA semantics, and demosaicing interface are
accepted, independent algorithm implementations should not block one another.

Decoder, metadata, preprocessing, and demosaicing tasks may overlap when their
interfaces are stable. Do not parallelize tasks that are still changing the same
shared API.

Only the `SYN-03` owner edits `agent-work/research/summary.md`. Each `RES-*`
owner edits only its assigned report. `SYN-01` and `SYN-02` write only their
own files under `agent-work/research/notes/`. Design work that consumes
synthesis output must cite the accepted `RES-*` or `SYN-*` task IDs.

If using multiple agents or parallel workers:

- Assign each worker a specific task ID and bounded file ownership.
- Require workers to use the accepted design and interfaces.
- Avoid concurrent edits to shared types and workspace manifests.
- Integrate through reviewable changes and run the relevant tests before marking tasks complete.

Identify which tasks are ready to run in parallel and which are blocked by design decisions.

## 5. Phase D — Implementation

Implement against the accepted research and design documents.

Requirements:

- Reuse existing `fornax` code where appropriate.
- Avoid unnecessary crate proliferation or dependency additions.
- Keep decoding, metadata interpretation, preprocessing, and demosaicing separate.
- Define numerical behavior explicitly.
- Preserve metadata through stage transitions.
- Use structured errors for invalid layouts, unsupported formats, missing required metadata, and unsupported CFA patterns.
- Avoid hidden post-processing.
- Prefer safe Rust and document necessary unsafe code.
- Preserve existing public APIs where practical and document intentional breaking changes.
- Update the implementation tracker as work proceeds.

Do not silently change shared interfaces to accommodate one algorithm. If the design must change, update the design report and affected task contracts before proceeding.

## 6. Phase E — Testing and validation

The test plan must cover:

1. Unit tests for metadata interpretation, CFA indexing, buffer layout, and preprocessing.
2. Synthetic mosaics with deterministic expected outputs.
3. Real RAW integration fixtures for supported formats and CFA patterns.
4. Comparisons with reference decoders and demosaicers where semantics are equivalent.
5. Numerical edge cases: black-level subtraction, clipping, saturation, borders, and missing metadata.
6. Metadata-preservation and pipeline-stage transition tests.
7. Regression tests for existing `fornax` behavior.
8. Performance tests for representative image sizes, allocations, and buffer copies.

Do not use visual plausibility as the only correctness criterion. Explain expected differences caused by decoder conventions, algorithm choices, or rounding.

Run the repository's applicable formatting, lint, test, and build commands. Record actual results and any checks that could not be run.

## 7. Execution protocol and final deliverables

Work in this order:

1. Start `RES-01` through `RES-06` as concurrent, tracked research tasks.
2. Have each owner record findings and evidence in exactly one report under
   `agent-work/research/`.
3. Run `SYN-01` after its prerequisite reports are accepted, then `SYN-02`
   after `SYN-01`, then `SYN-03` to produce `agent-work/research/summary.md`.
4. Review the reports and the synthesis, resolving or documenting disagreements.
5. Produce the API and data structure design report (`DES-001`–`DES-006`),
   citing `RES-*` or `SYN-*` evidence for every material decision.
6. Review the architecture (`DES-007`). No public type or preprocessing API is
   finalized before `SYN-03` is accepted.
7. Implement in small, trackable tasks only after design acceptance.
8. Integrate, validate, and update documentation.

Research tasks, synthesis tasks, design tasks, and implementation tasks are all
tracked in `agent-work/tasks/raw-pipeline.md`. Update status and evidence as
work proceeds. Do not begin implementation while any required synthesis or
design gate is unresolved.

At the end of the research/design phase, provide a concise review summary and identify the tasks that are ready for implementation. Do not treat research or design documents as proof that the implementation exists.

At the end of implementation, report:

- The final architecture and important research findings.
- The accepted API and data model.
- How metadata drives preprocessing before demosaicing.
- Task status, including blocked and incomplete work.
- Which tasks were implemented concurrently.
- Actual test and validation results.
- Unsupported formats, known limitations, and remaining work.

The goal is a maintainable, Rust-native RAW pipeline whose data semantics are explicit and whose implementation can be developed incrementally by multiple independent contributors.
