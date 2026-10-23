# Task: Research, Design, and Implement a Rust-Native RAW Processing Pipeline in `fornax`

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

### Research deliverable

Create `docs/research/raw-pipeline.md`.

It must contain:

- A summary of each reference implementation.
- A comparison table covering decoding, sample semantics, metadata, preprocessing, and demosaicing.
- The relevant source files and symbols for important findings.
- Examples of metadata as represented by upstream implementations.
- An explanation of differences between formats and libraries.
- Identified gaps in the current `fornax` implementation.
- Recommended initial format, camera, and CFA support.
- Evidence-based conclusions and unresolved questions.

Do not begin a broad implementation before the research report contains enough evidence to support the proposed architecture.

## 3. Phase B — API and data structure design

Create `docs/design/raw-pipeline.md` as a detailed design proposal.

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

## 4. Phase C — Tracked implementation backlog

Create `docs/implementation/roadmap.md` as a living implementation tracker.

Every implementation task must have:

- A stable task ID.
- A concise title and scope.
- Status: `blocked`, `ready`, `in_progress`, `review`, or `done`.
- Dependencies expressed as task IDs.
- Acceptance criteria.
- Relevant crates and files.
- Parallelization notes.
- Test and validation requirements.

Use Markdown checklists or tables that are easy to update in code review. Keep the tracker synchronized with actual implementation progress; never mark work complete without evidence.

### Initial work breakdown

Create and refine tasks covering at least the following workstreams.

**A. Architecture and shared contracts**

- Repository audit and reference research.
- Review and acceptance of the API/data model design.
- Universal RAW representation and common metadata.
- Extensible metadata and provenance.
- CFA representation and indexing semantics.
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

Represent dependencies explicitly. A typical dependency graph is:

`Research → Design approval → Shared contracts → Independent implementation tasks → Integration`

After the common data model, CFA semantics, and demosaicing interface are accepted, independent algorithm implementations should not block one another.

Decoder, metadata, preprocessing, and demosaicing tasks may overlap when their interfaces are stable. Do not parallelize tasks that are still changing the same shared API.

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

1. Audit the existing repository.
2. Research the reference implementations.
3. Produce the research report.
4. Produce the API and data structure design report, emphasizing metadata semantics and preprocessing.
5. Produce the implementation tracker with task dependencies and parallelization notes.
6. Review the architecture and resolve critical design questions.
7. Implement in small, trackable tasks.
8. Integrate, validate, and update documentation.

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
