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
> │   ├── raw-pipeline.md          # consolidated report
> │   └── notes/                   # per-task working notes
> │       ├── RSCH-001.md
> │       └── ...
> ├── design/raw-pipeline.md
> └── tasks/raw-pipeline.md
> ```
>
> The consolidated research report is `agent-work/research/raw-pipeline.md`.
> Per-task working notes live under `agent-work/research/notes/` so that
> parallel research workers never edit the consolidated report concurrently.
>
> These reports are future deliverables. Do not fabricate them during
> initialization, and do not begin the research phase until instructed.


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

### Research is a tracked phase

Research is not an untracked preamble. It is decomposed into independently
verifiable tasks with stable IDs, explicit dependencies, acceptance criteria,
and a single consolidated deliverable. Live status is maintained in
`agent-work/tasks/raw-pipeline.md`; this section defines the normative scope.

| ID | Research target | Depends on | Parallel group | Deliverable |
| --- | --- | --- | --- | --- |
| `RSCH-001` | Existing `fornax` architecture and reusable code | none | R0 | `agent-work/research/notes/RSCH-001.md` + consolidated section |
| `RSCH-002` | LibRaw decoding, sample semantics, and metadata | none | R0 | `agent-work/research/notes/RSCH-002.md` + consolidated section |
| `RSCH-003` | dnglab and DNG metadata interpretation | none | R0 | `agent-work/research/notes/RSCH-003.md` + consolidated section |
| `RSCH-004` | darktable preprocessing and black-level handling | none | R0 | `agent-work/research/notes/RSCH-004.md` + consolidated section |
| `RSCH-005` | Demosaicing algorithms and shared input/output contracts | none | R0 | `agent-work/research/notes/RSCH-005.md` + consolidated section |
| `RSCH-006` | Other relevant Rust RAW libraries and implementations | none | R0 | `agent-work/research/notes/RSCH-006.md` + consolidated section |

All six research tasks are independent and may run concurrently. `RSCH-006`
must not duplicate `RSCH-002` or `RSCH-003`; it covers libraries beyond LibRaw
and dnglab.

#### `RSCH-001` — Existing `fornax` architecture and reusable code

- **Scope:** Audit workspace and crate manifests, dependency direction,
  features, build scripts, CI, tests, fixtures, benchmarks, error types,
  naming conventions, and coding standards. Inventory `fornax-core`,
  `fornax-dalim`, `libraw`, `libraw-sys`, `fornax`, and `fornax-py`.
- **Acceptance criteria:** Every existing public trait and type relevant to
  decoding, metadata, preprocessing, or demosaicing is listed with file path
  and symbol. Reuse candidates and incompatibilities are identified. The
  current `BayerImage`/`IDemosaic` limitations are stated explicitly.
- **Deliverable:** Audit section in the consolidated report, with a reuse
  matrix and a list of gaps that the universal representation must close.
- **Parallelization:** Independent; start immediately.

#### `RSCH-002` — LibRaw decoding, sample semantics, and metadata

- **Scope:** Trace LibRaw from file opening through unpacking and raw-image
  exposure. Determine what `rawdata.raw_image`, `color4_image`, dimensions,
  `filters`, black levels, white levels, saturation, crop, and margins mean.
  Distinguish sensor codes from linearized or processed samples.
- **Acceptance criteria:** Each relevant LibRaw field is documented with unit,
  coordinate convention, sentinel values, and the exact upstream source path
  and symbol. The black-level repeat-pattern semantics are explained. Claims
  that cannot be verified from source are marked as assumptions.
- **Deliverable:** Source-verified LibRaw section with metadata examples and
  a field-to-universal-candidate mapping.
- **Parallelization:** Independent; start immediately.

#### `RSCH-003` — dnglab and DNG metadata interpretation

- **Scope:** Inspect dnglab's DNG parsing, `RawImage` representation, CFA
  handling, black/white level interpretation, crop/orientation metadata, and
  decoder output semantics. Compare with the DNG specification where the
  source is ambiguous.
- **Acceptance criteria:** DNG tags relevant to sample interpretation and
  preprocessing are mapped to source symbols, with units, defaults, and
  coordinate conventions. Differences between DNG metadata and vendor RAW
  metadata are identified. Licensing constraints are recorded.
- **Deliverable:** Source-verified dnglab/DNG section and a tag mapping table.
- **Parallelization:** Independent; start immediately.

#### `RSCH-004` — darktable preprocessing and black-level handling

- **Scope:** Inspect darktable's rawprepare, temperature, demosaic, and related
  pipeline modules. Determine the order of sensor-domain operations, how
  black/white levels and masks are represented, and which operations occur
  before demosaicing. Verify the exact project and repository intended by any
  reference to "darkroom".
- **Acceptance criteria:** The pre-demosaic pipeline order is documented with
  source paths and symbols. Black-level subtraction, clipping, normalization,
  and masked-pixel handling are described numerically. Any operation that is
  out of scope for `fornax` is named and excluded with a reason.
- **Deliverable:** Source-verified darktable section and a proposed
  pre-demosaic operation sequence for evaluation.
- **Parallelization:** Independent; start immediately.

#### `RSCH-005` — Demosaicing algorithms and shared input/output contracts

- **Scope:** Survey bilinear, Malvar–He–Cutler, and representative edge-aware
  or higher-quality algorithms. For each, record CFA assumptions, border
  handling, output channel semantics, numeric range, and metadata needs.
  Inspect existing `fornax-dalim` implementations and benchmarks.
- **Acceptance criteria:** Candidate algorithms are compared on input
  assumptions, output semantics, complexity, and testability. A recommended
  initial set is justified. The minimum shared input contract required by all
  candidates is stated without committing to a final API.
- **Deliverable:** Algorithm survey, contract requirements, and a ranked
  recommendation in the consolidated report.
- **Parallelization:** Independent; start immediately. Algorithm
  implementations remain blocked until the shared contract is accepted.

#### `RSCH-006` — Other relevant Rust RAW libraries and implementations

- **Scope:** Survey additional Rust RAW decoders, DNG/TIFF libraries, and
  demosaicing implementations not covered by `RSCH-002` or `RSCH-003`.
  Include rawloader, rawler, image-rs decoders, and other maintained projects
  where relevant. Record maintenance status and licensing.
- **Acceptance criteria:** Each surveyed project has a source-verified summary
  of its sample representation, metadata model, CFA handling, and
  preprocessing. Reusable ideas and code are distinguished, with license
  constraints recorded. The section states which findings are assumptions.
- **Deliverable:** Comparative section and a shortlist of ideas worth adopting.
- **Parallelization:** Independent; start immediately. Coordinate with
  `RSCH-002` and `RSCH-003` to avoid duplicate coverage.

### Evidence and provenance rules

Every research task must:

- Cite upstream repository URL, commit or version, source path, and symbol for
  each material claim. Cite tests or documentation when they are the evidence.
- Label each finding as **verified** (observed in source, tests, or docs),
  **assumption** (inferred but unverified), or **recommendation** (a proposed
  choice for `fornax`).
- Record licensing constraints for any code considered for reuse.
- Record unresolved questions and contradictions between sources.
- Avoid relying on README descriptions alone.
- Write working notes only to its own `agent-work/research/notes/<ID>.md`.
  Workers must not edit the consolidated report concurrently; the synthesis
  owner integrates accepted findings.

### Research synthesis gates

Independent findings are not directly actionable until they are reconciled.
After the six research tasks reach `review`, the following synthesis tasks
must run before any universal representation or public API is proposed.

| ID | Reconciles | Depends on | Produces |
| --- | --- | --- | --- |
| `SYNTH-001` | Metadata fields, units, provenance, and authority rules | `RSCH-002`, `RSCH-003`, `RSCH-004`, `RSCH-006` | Normative metadata field catalogue |
| `SYNTH-002` | Coordinate systems, crop, margins, active area, and CFA phase | `RSCH-002`, `RSCH-003`, `SYNTH-001` | Coordinate and CFA-origin contract |
| `SYNTH-003` | Black-level semantics, clipping, normalization, and pre-demosaic order | `RSCH-002`, `RSCH-004`, `SYNTH-001`, `SYNTH-002` | Preprocessing semantics contract |
| `SYNTH-004` | Sample ranges, linearization, and decoder output semantics | `RSCH-001`, `RSCH-002`, `RSCH-003`, `RSCH-006` | Sample-domain and decoder contract |
| `SYNTH-005` | Demosaicing input/output requirements across algorithms | `RSCH-005`, `SYNTH-002`, `SYNTH-004` | Demosaicing interface requirements |

`SYNTH-001`, `SYNTH-002`, and `SYNTH-004` may begin concurrently once their
research dependencies are accepted. `SYNTH-003` waits for `SYNTH-001` and
`SYNTH-002`. `SYNTH-005` waits for `SYNTH-002` and `SYNTH-004`.

Each synthesis task must state, for every reconciled item:

- The chosen meaning, unit, coordinate convention, and valid range.
- The authority rule when sources disagree.
- Which findings are verified, assumed, or recommended.
- Which open questions remain and which design task is blocked by them.

The synthesis outputs are inputs to Phase B. Do not propose the universal RAW
representation or public APIs before the relevant synthesis tasks are accepted.

### Research deliverable

Create `agent-work/research/raw-pipeline.md` as the single consolidated report.
It must contain:

- A summary of each reference implementation.
- A comparison table covering decoding, sample semantics, metadata,
  preprocessing, and demosaicing.
- The relevant source files, symbols, tests, and documentation for important
  findings.
- Examples of metadata as represented by upstream implementations.
- An explanation of differences between formats and libraries.
- Identified gaps in the current `fornax` implementation.
- Recommended initial format, camera, and CFA support.
- The synthesis decisions for metadata, coordinates, black levels, sample
  ranges, and decoder behavior.
- Evidence-based conclusions, assumptions, recommendations, and unresolved
  questions.

Per-task working notes belong in `agent-work/research/notes/` and are
supporting evidence, not substitutes for the consolidated report.

Do not begin a broad implementation before the research report and the
relevant synthesis gates are accepted.

## 3. Phase B — API and data structure design

Create `agent-work/design/raw-pipeline.md` as a detailed design proposal.

Design work is tracked as `DES-001` through `DES-006`, followed by the
`DES-007` review gate. A design task may start only when its listed `SYNTH-*`
dependencies are accepted, and its proposal must cite the synthesis task IDs
that justify each decision. Do not introduce a public type or function whose
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
| `RSCH-` | Independent reference research | `ready` |
| `SYNTH-` | Cross-source synthesis and reconciliation | `blocked` |
| `DES-` | API, data model, and architecture design | `blocked` |
| `IMPL-` | Implementation | `blocked` |
| `QA-` | Integration, validation, and performance | `blocked` |

### Initial work breakdown

Create and refine tasks covering at least the following workstreams.

**A. Research, synthesis, and shared contracts**

- `RSCH-001` through `RSCH-006`: the six independent research tasks defined in
  Section 2. All are `ready` at initialization and may run concurrently.
- `SYNTH-001` through `SYNTH-005`: reconcile metadata fields, units, coordinate
  systems, black-level semantics, sample ranges, and decoder behavior before
  design. Each is `blocked` on its listed research tasks.
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

`RSCH-001..006 → SYNTH-001..005 → DES-001..007 → IMPL-* → QA-*`

Execution waves:

- **Wave R0 — independent research.** `RSCH-001` through `RSCH-006` are
  `ready` simultaneously. Each worker owns one task ID, one working-notes file,
  and no shared report edits.
- **Wave S1 — first synthesis.** `SYNTH-001`, `SYNTH-002`, and `SYNTH-004` may
  run concurrently once their research dependencies are in `review` or `done`.
- **Wave S2 — dependent synthesis.** `SYNTH-003` waits for `SYNTH-001` and
  `SYNTH-002`. `SYNTH-005` waits for `SYNTH-002` and `SYNTH-004`.
- **Wave D — design.** `DES-001` through `DES-006` may overlap only where their
  synthesis inputs are accepted. `DES-007` is the single design-review gate and
  depends on all design tasks.
- **Wave I — implementation.** No `IMPL-*` task is `ready` until `DES-007` is
  accepted and the affected interfaces are stable.

After the common data model, CFA semantics, and demosaicing interface are
accepted, independent algorithm implementations should not block one another.

Decoder, metadata, preprocessing, and demosaicing tasks may overlap when their
interfaces are stable. Do not parallelize tasks that are still changing the same
shared API.

Only the synthesis integrator edits `agent-work/research/raw-pipeline.md`.
Research workers write `agent-work/research/notes/<ID>.md`. Design work that
consumes synthesis output must cite the accepted `SYNTH-*` task IDs.

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

1. Start `RSCH-001` through `RSCH-006` as concurrent, tracked research tasks.
2. Have each research worker record evidence and citations in its working-notes
   file and hand its section to the synthesis integrator.
3. Run `SYNTH-001`, `SYNTH-002`, and `SYNTH-004` once their research
   dependencies are accepted; then run `SYNTH-003` and `SYNTH-005`.
4. Produce the consolidated research report including synthesis decisions.
5. Produce the API and data structure design report (`DES-001`–`DES-006`),
   emphasizing metadata semantics and preprocessing, citing accepted synthesis
   task IDs.
6. Review the architecture (`DES-007`) and resolve critical design questions.
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
