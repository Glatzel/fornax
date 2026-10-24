# Task: Research, Design, and Implement a Rust-Native RAW Processing Pipeline in `fornax`

> **Authority:** This file is the authoritative workflow and task specification
> for the RAW processing pipeline. `AGENTS.md` at the repository root contains
> only concise repository instructions and must not duplicate or override this
> specification.
>
> **Task contracts:** One TOML manifest per task lives in
> `agent-work/tasks/raw-pipeline/<TASK-ID>.toml`. GitHub pull requests are the
> review and merge record. `agent-work/tasks/STATUS.md` is the human-readable
> task-status snapshot; manifests contain no duplicate live review state.
>
> **Deliverable layout:** The original prompt used `docs/` paths. Those paths are
> mapped to this repository's `agent-work/` directory. The hidden `.agents/`
> directory is mounted read-only in the agent runtime, so `agent-work/` is the
> canonical location for agent materials.
>
> ```text
> agent-work/
> ├── AGENTS.md
> ├── README.md
> ├── workflows/raw-pipeline.md
> ├── references/
> │   ├── README.md
> │   ├── darktable/
> │   ├── dnglab/
> │   └── other/
> ├── checkpoints/
> │   ├── README.md
> │   ├── workflow/
> │   └── <TASK-ID>/<date>-<slug>.md
> ├── research/
> │   ├── README.md
> │   ├── RES-01-fornax-inventory.md
> │   ├── RES-02-libraw.md
> │   ├── RES-03-dnglab.md
> │   ├── RES-04-darktable.md
> │   ├── RES-05-demosaicing.md
> │   ├── RES-06-rust-raw-decoders.md
> │   ├── RES-07-rust-dng-tiff.md
> │   ├── RES-08-rust-demosaicing.md
> │   ├── RES-09-rust-ecosystem.md
> │   ├── SYN-03-summary.md
> │   └── notes/                   # synthesis working notes
> ├── design/
> │   └── raw-pipeline/<TASK-ID>-<slug>.md
> └── tasks/
>     ├── README.md               # manifest and PR protocol
>     ├── STATUS.md               # task-status snapshot
>     └── raw-pipeline/
>         ├── README.md           # planned dependency graph
>         └── <TASK-ID>.toml      # one contract per task
> ```
>
> Each reference report has exactly one owning research task. `SYN-03-summary.md` is
> produced by `SYN-03`; it synthesizes the reports rather than replacing them.
> A file existing is not evidence of completion, and no task may fabricate
> findings or bypass its acceptance gate. Review approval and merge state are
> recorded by the task's GitHub pull request; task progress is mirrored in
> `agent-work/tasks/STATUS.md`.
> Every task-owned research filename must begin with its task ID
> (`<TASK-ID>-<slug>.md`); only the directory-level `README.md` is exempt.

## 0. Agent workspace and reference sources

`agent-work/AGENTS.md` contains the detailed operating instructions. The root
`AGENTS.md` points to it and to this authoritative workflow so repository-level
instruction discovery remains reliable.

Upstream repositories used as evidence live under `agent-work/references/`.
They are read-only evidence sources, not project instructions, and must not be
added to the Cargo workspace or production builds. They are pinned Git
submodules so a fresh checkout can reproduce the evidence and a maintainer can
review upstream updates deliberately. The tracked
`agent-work/references/README.md` records each upstream URL, branch, pinned
commit, license, purpose, and any exception. Verify a checkout with
`git -C <path> rev-parse HEAD` and `git -C <path> status --short` before using
it. Do not move a reference to a new revision in the middle of a task without
recording the change in the owning report. Update the submodule gitlink,
manifest revision, and affected reports together; `git submodule update
--remote` alone is not a completed reference update.

The existing `external/raw-images` submodule is a production test-fixture
dependency, not a research reference. Keep it at its current path.

Research owners write only their assigned report under `agent-work/research/`.
`SYN-01` and `SYN-02` write only their files under
`agent-work/research/notes/`; `SYN-03` owns
`agent-work/research/SYN-03-summary.md`. Each `DES-*` task writes a separate
proposal under `agent-work/design/raw-pipeline/`. The normative persistent
design lives in repository-root `docs/raw-pipeline/`. No task writes into
another task's owned file.

## 1. Project context and objective

`fornax` is an existing Rust RAW image processing repository containing crates related to LibRaw bindings, Rust demosaicing implementations, Python bindings, and related functionality.

Replace the existing architecture with a Rust-native RAW processing design.
Treat the current repository as legacy input: inventory its components and
identify implementation assets that may be reusable, but do not preserve its
crate boundaries, traits, composition model, or public API unless the new design
independently justifies them. Relevant upstream RAW libraries remain evidence,
not architectural templates.

Decoder reuse is an architectural requirement, not a convenience. Expose
existing RAW decoders through selectable, backend-neutral adapters; do not make
a first-party RAW format decoder the initial design or implementation target.
The shared model, capability contract, selection policy, and provenance belong
to `fornax`; format parsing, vendor metadata tables, and codec machinery stay in
the selected backend.

The target pipeline is:

1. **Decode:** Read a supported RAW file through the selected existing-decoder
   adapter and produce a universal, backend-independent RAW representation with
   sufficient metadata for subsequent processing.
2. **Preprocess:** Interpret the decoded sensor metadata and perform explicitly defined RAW-domain corrections, especially black-level correction.
3. **Demosaic:** Reconstruct a full-resolution image from the sensor mosaic using interchangeable algorithms.

The pipeline ends at demosaicing. Do not implement downstream white balance application, color correction, color-space conversion, tone mapping, denoising, sharpening, or display rendering.

The primary deliverable is a well-researched and documented design, followed by an implementation whose progress and dependencies are explicitly tracked.

### Initial sensor and file-format scope

The initial pipeline targets a single-plane, conventional 2x2 RGB Bayer mosaic.
The four Bayer phases (`RGGB`, `BGGR`, `GRBG`, and `GBRG`) are in scope, with
CFA origin and crop alignment represented explicitly. Every other mosaic or
sensor layout must produce a structured unsupported-layout error.

The initial file-format target is:

| Priority | Format | Initial target |
| --- | --- | --- |
| P0 | DNG | Standard TIFF/EP-based Bayer DNG |
| P0 | Sony ARW | Sony Bayer RAW |
| P0 | Nikon NEF | Nikon Bayer RAW |
| P0 | Canon `CR*` | Canon Bayer RAW, with `CR2` and `CR3` expected in the regression matrix |

The following are explicitly deferred and documented as future work, not
implicitly supported:

- Fujifilm X-Trans sensors and RAF files.
- Sigma Foveon/X3 sensors and X3F files.
- Quad Bayer, four-color CFA, monochrome, linearized/non-mosaic, and any other
  non-conventional-Bayer payload.
- RAW formats other than DNG, Sony ARW, Nikon NEF, and Canon `CR*`.

Research may inspect non-target layouts to understand why they must be rejected,
but no initial task is required to decode, preprocess, or demosaic them.

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
status is maintained by the task pull requests. This section defines the
normative research scope; the TOML manifest is authoritative for each task
contract.

| ID | Report | Focus | Depends on | Parallel |
| --- | --- | --- | --- | --- |
| `RES-01` | `agent-work/research/RES-01-fornax-inventory.md` | Existing components, current contracts, implementation assets reusable in the replacement, and refactor hazards | none | Yes, wave R0 |
| `RES-02` | `agent-work/research/RES-02-libraw.md` | LibRaw decoding, sample semantics, CFA, black/white levels, metadata, APIs | none | Yes, wave R0 |
| `RES-03` | `agent-work/research/RES-03-dnglab.md` | DNG metadata, tags, active area, black-level patterns, white levels, coordinates | none | Yes, wave R0 |
| `RES-04` | `agent-work/research/RES-04-darktable.md` | RAW preprocessing pipeline, black-level correction, CFA handling, processing boundary | none | Yes, wave R0 |
| `RES-05` | `agent-work/research/RES-05-demosaicing.md` | Demosaicing algorithms, inputs, CFA handling, output semantics, common interfaces | none | Yes, wave R0 |
| `RES-06` | `agent-work/research/RES-06-rust-raw-decoders.md` | Rust RAW decoder crates, representations, target-format coverage, maintenance, licenses | none | Yes, wave R0 |
| `RES-07` | `agent-work/research/RES-07-rust-dng-tiff.md` | Rust TIFF/DNG building blocks, sample layout, errors, dependency suitability | none | Yes, wave R0 |
| `RES-08` | `agent-work/research/RES-08-rust-demosaicing.md` | Rust demosaicing crates, Bayer input/output contracts, validation, test patterns | none | Yes, wave R0 |
| `RES-09` | `agent-work/research/RES-09-rust-ecosystem.md` | Existing-decoder and Rust ecosystem comparison, adapter boundaries, backend selection, reuse, and license recommendation | `RES-01`–`RES-04`, `RES-06`–`RES-08` | After R0 |

Each research task owns exactly one report. A task may reference another report,
but it must not edit it. `RES-01`–`RES-08` form independent wave R0 and may run
concurrently. `RES-09` compares the relevant backend and Rust ecosystem reports
after they are ready.

#### RES-01 — Inventory existing `fornax` components and reusable assets

- **Output report:** `agent-work/research/RES-01-fornax-inventory.md`
- **Scope:** Inventory what currently exists and identify what may be reusable
  during implementation of the replacement architecture. Cover the workspace
  crates, public contracts, concrete implementations, tests, fixtures,
  benchmarks, build/tooling, and relevant safety or defect findings. Inventory
  `fornax-core`, `fornax-dalim`, `libraw`, `libraw-sys`, `fornax`, `fornax-py`,
  `dnc`, and `fornax-devtool`. Determine what `IDecoder`, `IPostProcessor`,
  `BayerImage`, `BayerPattern`, `IDemosaic`, and `DemosaicLinear` actually
  guarantee. The current architecture and public APIs are not design
  constraints.
- **Evidence requirements:** Cite local file paths, line ranges, symbols, and
  test names. Record Cargo manifests, features, and dependency edges. Do not
  infer public behavior from names alone.
- **Acceptance criteria:** Relevant existing components and public contracts are
  inventoried with paths and symbols. Current limitations and defects are
  explicit. Reusable implementation assets are classified as direct reuse,
  adaptation, test/oracle only, or not reusable, with risks and expected
  refactor work. The report states that the architecture will be fully
  refactored and does not recommend preserving crate boundaries or API shape.
  The report satisfies the required report structure.
- **Parallelization:** Independent. Owns only
  `agent-work/research/RES-01-fornax-inventory.md`.

#### RES-02 — Research LibRaw

- **Output report:** `agent-work/research/RES-02-libraw.md`
- **Scope:** Trace LibRaw from file opening through unpacking and raw-image
  exposure. Establish the meaning of `rawdata.raw_image`, `color4_image`,
  dimensions, `filters`, black levels, white levels, saturation, crop, masks,
  and margins. Distinguish original sensor codes from unpacked, linearized, or
  already processed samples. Prioritize behavior needed for DNG, Sony ARW, Nikon
  NEF, and Canon `CR*` Bayer images. Record the relevant public APIs and the
  behavior of the existing `libraw` wrapper.
- **Evidence requirements:** Cite upstream repository URL, version or commit,
  source path, symbol, line range, and tests or documentation where applicable.
  Inspect implementation code rather than relying on README prose. Record
  licensing constraints for considered reuse.
- **Acceptance criteria:** Each relevant field is documented with type, unit,
  coordinate convention, sentinel values, and validity conditions. Black-level
  repeat-pattern and per-channel semantics are explained, including alignment
  with sensor coordinates. The difference between `raw_image` and processed
  output is stated. A field-to-universal-candidate mapping is provided. Coverage
  distinguishes the initial DNG/ARW/NEF/CR* target from deferred formats and
  non-Bayer payloads. Claims that cannot be verified are labeled `[inference]`.
  The report satisfies the required report structure.
- **Parallelization:** Independent. Owns only `agent-work/research/RES-02-libraw.md`.
  Coordinates scope with `RES-06` to avoid duplicate ecosystem coverage.

#### RES-03 — Research dnglab and DNG metadata

- **Output report:** `agent-work/research/RES-03-dnglab.md`
- **Scope:** Inspect dnglab's DNG parsing, raw-image representation, CFA
  handling, black/white level interpretation, active area, default crop,
  orientation, and decoder output semantics. Compare source behavior with the
  DNG specification where the source is ambiguous. Limit the supported target
  to standard Bayer DNG payloads; identify non-Bayer cases for explicit
  rejection and future work.
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
- **Parallelization:** Independent. Owns only `agent-work/research/RES-03-dnglab.md`.
  Coordinates scope with `RES-06` and `RES-07` to avoid duplicate ecosystem
  coverage.

#### RES-04 — Research darktable and RAW preprocessing

- **Output report:** `agent-work/research/RES-04-darktable.md`
- **Scope:** Inspect darktable's actual RAW preprocessing pipeline, including
  rawprepare, temperature, demosaic, and related modules. Determine the order of
  sensor-domain operations, how black and white levels and masks are
  represented, and where demosaicing sits. Use conventional Bayer behavior as
  the initial target. Verify the exact project and repository intended by any
  reference to "darkroom".
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
  `agent-work/research/RES-04-darktable.md`.

#### RES-05 — Research demosaicing algorithms and interfaces

- **Output report:** `agent-work/research/RES-05-demosaicing.md`
- **Scope:** Survey bilinear, Malvar–He–Cutler, and representative edge-aware or
  higher-quality algorithms. Record input requirements, CFA assumptions, border
  handling, output semantics, numeric range, metadata needs, and complexity.
  Inspect the existing `fornax-dalim` implementation and benchmark. Restrict
  candidate evaluation and the initial recommendation to conventional 2x2 RGB
  Bayer mosaics; record X-Trans, Foveon, and other layouts only as future work.
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
  `agent-work/research/RES-05-demosaicing.md`. Algorithm implementations remain blocked
  until design review.

#### RES-06 — Survey Rust RAW decoder crates

- **Output report:** `agent-work/research/RES-06-rust-raw-decoders.md`
- **Scope:** Evaluate Rust RAW decoder crates as adapters or evidence, initially
  rawloader and rawler's decoder-facing API. Record target-format coverage,
  stored sample representation, crop/CFA metadata, active/default crop,
  orientation, black/white metadata, preprocessing state, errors, and camera
  catalog shape. Keep DNG tag semantics in `RES-03`, TIFF internals in `RES-07`,
  and LibRaw internals in `RES-02`.
- **Evidence requirements:** Cite repository URL, commit, source paths,
  symbols, tests, and license files. Distinguish reusable code from reusable
  ideas. State whether coverage for DNG, Sony ARW, Nikon NEF, and Canon `CR*` was
  verified or remains unknown.
- **Acceptance criteria:** Each decoder crate has a source-verified sample and
  metadata model; target-format coverage and Bayer/non-Bayer behavior are
  explicit; maintenance, license, adapter risks, and unsupported cases are
  recorded; inferred claims are labeled `[inference]`; and the report contains
  the six required sections.
- **Parallelization:** Independent. Owns only
  `agent-work/research/RES-06-rust-raw-decoders.md`.

#### RES-07 — Survey Rust TIFF and DNG building blocks

- **Output report:** `agent-work/research/RES-07-rust-dng-tiff.md`
- **Scope:** Evaluate maintained Rust TIFF/DNG parsing and sample-layout
  components that could support the DNG adapter without duplicating `RES-03`
  tag semantics. Start with image-rs/image-tiff and any relevant DNG-facing API
  exposed by rawler. Assess sample types, strips/tiles, compression hooks,
  metadata access, row/plane strides, limits, errors, and extension points.
- **Evidence requirements:** Cite repository URL, commit, source paths,
  symbols, tests, and licenses. Distinguish what is verified for Bayer DNG from
  generic TIFF capability.
- **Acceptance criteria:** Candidate building blocks have source-verified
  capabilities and gaps for Bayer DNG; adding a dependency is compared with
  implementing the minimal parser; errors and limits are documented; deferred
  non-Bayer payloads are not presented as supported; and the report contains the
  six required sections.
- **Parallelization:** Independent. Owns only
  `agent-work/research/RES-07-rust-dng-tiff.md`. Coordinates with `RES-03` and
  `RES-06`.

#### RES-08 — Survey Rust demosaicing crates and interface patterns

- **Output report:** `agent-work/research/RES-08-rust-demosaicing.md`
- **Scope:** Evaluate maintained Rust demosaicing implementations and API
  patterns, including the `demosaic` crate and MeraRAW Demosaic. Focus on
  conventional Bayer input/output, border policy, validation, errors, strategy
  registration, and synthetic-test patterns. Coordinate algorithm quality and
  the initial algorithm set with `RES-05`.
- **Evidence requirements:** Cite repository URL, commit, source paths,
  symbols, tests, and licenses. Separate reusable code from reusable interface
  ideas.
- **Acceptance criteria:** Bayer API contracts, borders, validation, errors,
  maintenance, licenses, and reusable test patterns are source-verified;
  X-Trans/Foveon/Quad Bayer support is noted but excluded from the initial
  recommendation; and the report contains the six required sections.
- **Parallelization:** Independent. Owns only
  `agent-work/research/RES-08-rust-demosaicing.md`.

#### RES-09 — Compare existing decoder and Rust ecosystem candidates

- **Output report:** `agent-work/research/RES-09-rust-ecosystem.md`
- **Scope:** Compare the existing decoder findings from `RES-01`–`RES-04` and
  the Rust ecosystem findings from `RES-06`–`RES-08` without repeating their
  evidence. Produce a decision matrix for LibRaw, dnglab/rawler, `dnc`,
  darktable/rawspeed, DNG/TIFF reuse, and demosaicing interfaces; recommend
  which components are selectable backends, conversion bridges, dependencies,
  design references, or unsuitable.
- **Evidence requirements:** Link every conclusion to the split reports. Keep
  source citations in the owned reports and call out license or maintenance
  conflicts explicitly.
- **Acceptance criteria:** Every decoder, conversion bridge, and reusable
  component is compared on scope fit, Bayer/target-format fit, API boundary,
  maintenance, and license; a backend-neutral adapter contract specifies
  explicit selection, capabilities, structured rejection, and provenance; no
  whole upstream RAW type is recommended for the `fornax` public model; the recommendation
  does not require `fornax` to maintain a first-party RAW decoder; all
  recommendations are traceable to the split reports; and the report contains
  the six required sections.
- **Parallelization:** Starts after `RES-01`–`RES-04` and `RES-06`–`RES-08` are
  approved and merged in their dependency task PRs. Owns only
  `agent-work/research/RES-09-rust-ecosystem.md`.

### Required report structure

Every reference report (`RES-01-fornax-inventory.md`, `RES-02-libraw.md`, `RES-03-dnglab.md`,
`RES-04-darktable.md`, `RES-05-demosaicing.md`, `RES-06-rust-raw-decoders.md`,
`RES-07-rust-dng-tiff.md`, `RES-08-rust-demosaicing.md`, `RES-09-rust-ecosystem.md`) must contain:

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

Front matter must record the owning task ID, owner, last update, upstream
versions or commits inspected, and a link or identifier for the task pull
request once one exists. Each report must include the acceptance criteria of
its owning task manifest. Agents submit the report through the task PR; only
the user approves and merges it.

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
prerequisite reports are approved and merged in the dependency task PR. It must not silently
reconcile disagreements.

| ID | Output | Reconciles | Depends on |
| --- | --- | --- | --- |
| `SYN-01` | `agent-work/research/notes/SYN-01.md` | Metadata and sample semantics | `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06`, `RES-07` |
| `SYN-02` | `agent-work/research/notes/SYN-02.md` | Preprocessing order and black-level behavior | `RES-02`, `RES-04`, `SYN-01` |
| `SYN-03` | `agent-work/research/SYN-03-summary.md` | Cross-source synthesis and recommendations | `RES-01`–`RES-09`, `SYN-01`, `SYN-02` |

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
- Which operations are exercised by the initial Bayer DNG/ARW/NEF/CR* scope and
  which are explicitly deferred for other formats or sensor layouts.

It must produce an operation table with inputs, outputs, formulas, error
behavior, ordering constraints, and synthetic-test strategy.

`SYN-03` must:

- Compare findings across all nine reference reports.
- Document agreements, disagreements, uncertainties, and remaining questions.
- Make evidence-backed recommendations for the `fornax` architecture.
- Link each conclusion to the supporting report and section. It must not
  concatenate the reports or introduce unreferenced claims.
- State the assumptions behind every recommendation.
- Be reviewed before any design task is finalized.

### Research completion gate

- No synthesis task starts until every listed dependency report is
  approved and merged in the dependency task PR.
- No design task is finalized before `SYN-03` is approved and merged in the dependency task PR.
  Design decisions must cite the supporting merged `RES-*` or `SYN-*` task IDs,
  or explicitly state their assumptions.
- No implementation task starts before the relevant `DES-*` task PRs and the
  persistent design-docs PR are approved and merged.
- The research phase is complete only when `RES-01`–`RES-09` and `SYN-01`–
  `SYN-03` are approved and merged in the dependency task PR with satisfied acceptance criteria.
- A report file existing is not evidence of completion.

### Research deliverables

```text
agent-work/research/
├── README.md
├── RES-01-fornax-inventory.md
├── RES-02-libraw.md
├── RES-03-dnglab.md
├── RES-04-darktable.md
├── RES-05-demosaicing.md
├── RES-06-rust-raw-decoders.md
├── RES-07-rust-dng-tiff.md
├── RES-08-rust-demosaicing.md
├── RES-09-rust-ecosystem.md
├── SYN-03-summary.md
└── notes/
    ├── SYN-01.md
    └── SYN-02.md
```

`README.md` is the index of reports, owning tasks, status, dependencies, and
acceptance criteria. `SYN-03-summary.md` is produced only by `SYN-03`. Research owners
write only their own report; synthesis working notes are owned by `SYN-01` and
`SYN-02`.

Do not begin a broad implementation before the research reports and synthesis
are approved and merged in the dependency task PR.

## 3. Phase B — API and data structure design

Create one task-owned proposal under `agent-work/design/raw-pipeline/` for each
of `DES-001` through `DES-006`. The user-facing persistent design is maintained
under repository-root `docs/raw-pipeline/`; task-owned proposals retain design
rationale and review history.

Design work is tracked as `DES-001` through `DES-006`. A design task may start
only when its listed `SYN-*`
dependencies are approved and merged in the dependency task PR, and its proposal must cite the
merged `RES-*` or `SYN-*` task IDs that justify each decision. No public type or
function is finalized before `SYN-03` is approved and merged. Do not introduce a
public type or function whose metadata, coordinate, sample-domain, or
preprocessing semantics are still unresolved by synthesis.

The design must be consistent with the research report and the existing repository. It should describe actual proposed Rust types, fields, invariants, function signatures, data flow, and dependency boundaries rather than merely listing concepts.

Every design PR that changes user-facing or implementation-critical design must
update the relevant files under repository-root `docs/raw-pipeline/` in the same
PR. This includes pipeline order, precision, data structures, public contracts,
errors, allocation policy, or behavior. A task proposal alone is not a complete
design change. If a design document and task proposal differ, the persistent
`docs/raw-pipeline/` files are authoritative.

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

The initial implementation supports only the conventional Bayer layouts defined
in Section 1. Do not generalize shared behavior to X-Trans, Foveon, Quad Bayer,
four-color, monochrome, or non-mosaic payloads without a separately
user-approved and merged task. Define how those layouts and unusual sample encodings are
rejected with structured errors while keeping enough scope for a future
extension.

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

Design the replacement architecture from a clean-slate pipeline model. The
existing crate boundaries are not constraints; inspect them only to locate
reusable implementation assets and migration impact.

For each proposed or retained crate, document:

- Responsibility and public API.
- Dependencies and allowed dependency direction.
- Public versus internal types.
- Features and optional dependencies.
- Error handling.
- Whether it can be implemented and tested independently.

No existing crate is retained by default. Justify every retained or new crate
from its responsibility in the replacement design. Keep the core data model
independent of every decoder-specific type. Design existing RAW decoders as
selectable adapters; neither their APIs nor the current crate topology define
the replacement architecture.

LibRaw and dnglab/rawler are the initial decoder-backend candidates. The
existing `dnc` wrapper is a DNG-conversion bridge that must compose with a
decoder backend. darktable/rawspeed is eligible only if a decoder-only boundary
can return stored sensor data before `rawprepare`; the darktable application
pipeline must not become a backend. The design must define backend capabilities,
explicit selection, provenance, error translation, sample-domain state, and
dependency features without exposing backend types in the universal model.

Include representative Rust type definitions and function signatures in the design report.

## 4. Phase C — Tracked backlog and pull-request execution

The durable task plan is one TOML manifest per task at
`agent-work/tasks/raw-pipeline/<TASK-ID>.toml`. TOML is used for structured,
parseable data: dependencies, deliverables, acceptance criteria, and validation.
Markdown remains the format for task-owned evidence, design proposals,
checkpoints, and pull-request discussion.

One manifest per task avoids task-contract conflicts. The shared snapshot in
`agent-work/tasks/STATUS.md` tracks Blocked, Ready, and Done state for easier
human tracking. The task index in
`agent-work/tasks/raw-pipeline/README.md` records the dependency graph.

Mark a task `Done` when its deliverable is complete and its task PR is opened.
`Done` does not imply that the PR is merged. Keep downstream tasks `Blocked`
until every dependency PR is merged, then move them to `Ready`.

### Task manifest contract

Every task manifest declares:

- a stable task ID and immutable branch slug;
- title and phase;
- dependency task IDs;
- one or more task-owned deliverables;
- bounded scope;
- objective acceptance criteria; and
- commands or checks that produce validation evidence.

Task manifests must not contain `review`, `approved`, or `done` fields. Use
`agent-work/tasks/STATUS.md` for Blocked/Ready/Done tracking. GitHub pull
requests remain the sole review, approval, and merge record.

### Branch, worktree, and pull-request protocol

1. Verify every dependency task PR is merged into the integration base.
2. Fetch `origin` and create a worktree on a task branch:

   ```bash
   git worktree add -b task/impl-core-001-universal-core-model \
     .worktrees/impl-core-001-universal-core-model \
     origin/refactor/raw-pipeline-v2
   ```

3. Work only in the manifest deliverables and files needed to validate them.
   Keep unrelated cleanup in a separate task.
4. Commit and push the branch, then open one PR with a semantic title
   `<type>(<task-id>): <imperative summary>`, such as
   `feat(impl-core-001): implement universal raw model`, and use
   `agent-work/PULL_REQUEST_TEMPLATE.md` as its body. Implementation and QA PRs
   target `refactor/raw-pipeline-v2`. The type must be accepted by
   `.github/semantic.yml`, the scope must be the lowercase task ID, and the task
   ID must also appear in the PR body. Mark the task `Done` in
   `agent-work/tasks/STATUS.md`, add the PR link, and report the PR URL. Stop.
   Do not watch, poll, or wait for checks or review after creating it.
5. The user initiates the next step in conversation: request local changes or
   merge the PR. If changes are requested, update the same branch and PR unless
   the user asks for a local-only result. Do not open a second PR for the same
   task.
6. Agents never merge or close their own task PRs. A PR merged by the user is
   the durable completion record.
7. Remove the merged worktree and local branch only when cleanup is requested or
   completed by the normal housekeeping workflow.

`refactor/raw-pipeline-v2` is the long-lived integration branch for this large
breaking refactor. `IMPL-*` and `QA-*` branches start from its current `origin`
tip and merge back into it. Research, synthesis, and design tasks use `main`
unless the task index defines a stacked dependency. Do not propose the
integration branch to `main` until `QA-006` and every required implementation
PR are merged into it and validated there.

The pull request is the review and completion record, not a process the agent
must monitor. A paused task keeps its PR open and records resume context in
a `agent-work/checkpoints/<TASK-ID>/` checkpoint.

### Task ID scheme

| Prefix | Phase |
| --- | --- |
| `RES-` | Independent reference research |
| `SYN-` | Cross-source synthesis and reconciliation |
| `DES-` | API, data model, and architecture design |
| `IMPL-` | Implementation and integration |
| `QA-` | Validation, regression, performance, and documentation |

### Initial work breakdown

The post-design manifests cover these workstreams:

- independent source research and split Rust ecosystem research;
- metadata, preprocessing, and cross-source synthesis;
- universal representation, metadata, CFA, preprocessing, demosaicing, and crate
  architecture design;
- `IMPL-CORE-001` as the universal-model and direct stage-boundary gate in
  `fornax`;
- decoder registry, LibRaw, dnglab/rawler, DNC bridge, and decoder conformance
  modules (`IMPL-DEC-001`–`IMPL-DEC-005`);
- staged conversion/linearization, black subtraction, and
  normalization/clipping modules (`IMPL-PRE-001`–`IMPL-PRE-003`);
- direct demosaic dispatch, bilinear algorithm, and in-crate Malvar–He–Cutler
  module
  (`IMPL-DEM-001`–`IMPL-DEM-003`);
- end-to-end composition, Python/devtool migration, and legacy crate retirement
  (`IMPL-INT-001`–`IMPL-INT-003`); and
- metadata, numerical, error-path, performance, documentation, and workspace
  validation.

Tasks for additional preprocessing operations, general spatial black tiles and
deltas, or edge-aware demosaicing are intentionally absent. Those behaviors
remain deferred until a separate design decision and user-approved task add
them.

The exact scope and acceptance criteria are authoritative in each manifest, not
in a duplicated backlog table.

Run the manifest validator before opening or updating a task PR:

```bash
python3 agent-work/tasks/validate.py
```

### Dependency and parallelization rules

The completed research/design graph is:

`RES-01..08 -> SYN-01 -> SYN-02 -> SYN-03`

`RES-01..04,06..08 -> RES-09 -> SYN-03`

`SYN-03 -> DES-001..006 -> docs/raw-pipeline`

The implementation graph is:

`docs/raw-pipeline -> IMPL-CORE-001`

`IMPL-CORE-001 -> IMPL-DEC-001 | IMPL-PRE-001 | IMPL-DEM-001`

`IMPL-DEC-001 -> IMPL-DEC-002 | IMPL-DEC-003`

`IMPL-DEC-003 -> IMPL-DEC-004`

`IMPL-DEC-002 + IMPL-DEC-003 + IMPL-DEC-004 -> IMPL-DEC-005`

`IMPL-PRE-001 -> IMPL-PRE-002 -> IMPL-PRE-003`

`IMPL-DEM-001 -> IMPL-DEM-002 -> IMPL-DEM-003`

`IMPL-DEC-005 + IMPL-PRE-003 + IMPL-DEM-001 + IMPL-DEM-002 -> IMPL-INT-001`

`IMPL-INT-001 -> IMPL-INT-002`

`IMPL-INT-002 + all adapter/algorithm tasks -> IMPL-INT-003`

`IMPL-INT-001 -> QA-001..QA-004 -> QA-006`

`IMPL-INT-003 -> QA-005 -> QA-006`

An arrow starts downstream work only after the upstream task PR is merged.
Parallel workers must have distinct task IDs, branches, worktrees, and owned
files. Serialize edits to shared interfaces, workspace manifests, and the
persistent `docs/raw-pipeline/` design. If tasks need one shared file, introduce a stacked
dependency or a dedicated integration task instead of racing in separate PRs.

## 5. Phase D — Implementation

Implementation starts only after all relevant design task PRs and the
persistent `docs/raw-pipeline/` design PR are merged. The implementation is
planned around the boundaries that design actually produced:

1. `IMPL-CORE-001` renames the universal implementation to `fornax` and installs
   the model, metadata, CFA, image types, generic decoder boundary, direct
   stage methods, validation contracts, and reusable linear demosaic baseline.
   It also removes `fornax-dalim`.
2. The decoder track adds registry and private `libraw`, `rawler`, and optional
   DNC modules inside `fornax`, depending directly on upstream decoders behind
   optional features. `IMPL-DEC-005` validates conformance and target-format
   fixture claims.
3. The preprocessing track adds one private preprocessing implementation inside
   `fornax`, preserving the ordered substeps from decoded f32 conversion and
   linearization through black subtraction, normalization, and clipping.
4. The demosaic track refines direct enum dispatch and extends the initial
   linear baseline with additional in-crate algorithms inside `fornax`.
5. `IMPL-INT-001` composes explicit decoder and algorithm selection in the
   `fornax::pipeline` module. `IMPL-INT-002` migrates Python and devtool, and
   `IMPL-INT-003` retires the legacy public crates and architecture.

The cross-track contract is `IMPL-CORE-001`. A decoder, preprocessing, or
demosaic task may not fork the universal types or add a compatibility trait to
work around a missing core field. If implementation proves the core contract
insufficient, update the persistent design and affected task manifests in their
own reviewed change before continuing.

Requirements:

- Reuse existing `fornax` code where appropriate.
- Implement decoding by adapting the selected existing decoder backend. Do not
  add a first-party RAW format decoder within the initial implementation scope.
- Keep backend selection and capabilities explicit, preserve decoder
  provenance, and reject backend output that has already entered a processing
  pipeline.
- Avoid unnecessary crate proliferation or dependency additions.
- Keep decoding, metadata interpretation, preprocessing, and demosaicing separate.
- Define numerical behavior explicitly.
- Preserve metadata through stage transitions.
- Use structured errors for invalid layouts, unsupported formats, missing required metadata, and unsupported CFA patterns.
- Avoid hidden post-processing.
- Prefer safe Rust and document necessary unsafe code.
- Use applicable `glatzel-*` skills during implementation:
  `glatzel-preferred-rust-crates` for dependency selection,
  `glatzel-rust-apidoc` for public Rust API documentation, and
  `glatzel-tool-discovery` when installed local tooling can materially help.
  These skills are discovered automatically from the Pixi `agent` environment:
  its `codex` package exports `CODEX_HOME`, and `skill-glatzel` installs the
  skill packages there. Do not add a separate skill setup or copy step; refresh
  the environment with `pixi install -e agent` when the lockfile changes.
  Skills do not override the task manifest, persistent design, or review gates.
- Treat the architecture refactor as breaking by default. Preserve only
  implementation assets that independently pass the new design and license
  requirements, and document their migration.
- Record implementation evidence in the task PR as work proceeds.

Do not silently change shared interfaces to accommodate one algorithm. If the design must change, update the design report and affected task contracts before proceeding.

Shared-file ownership is explicit:

- decoder tasks own distinct modules under `fornax::decoder` and must not edit
  another decoder module's source;
- preprocessing transitions are serialized by the PRE dependency chain;
- demosaic algorithm additions are serialized by the DEM dependency chain;
- only the integration tasks may change the pipeline composition and feature
  topology after their listed components merge; and
- remaining legacy crate removal occurs in `IMPL-INT-003`, after replacement
  consumers and tests exist.

Parallel component tasks may update the shared root `Cargo.toml` or generated
`Cargo.lock`. When that happens, merge one branch first and require the other
task branches to rebase and regenerate the lockfile before updating their PRs;
do not resolve generated-lock conflicts by hand.

General spatial black tiles/vectors, additional sensor-domain transforms, and
edge-aware demosaicing are not implementation tasks under this plan. They need a
new approved design and manifest before implementation.

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

The initial real-file acceptance matrix is DNG, Sony ARW, Nikon NEF, and Canon
`CR*` (including `CR2` and `CR3` coverage), using only conventional Bayer
payloads. Test fixtures for Fujifilm X-Trans, Sigma Foveon, other non-Bayer
layouts, and deferred formats are rejection tests, not supported-format tests.

Do not use visual plausibility as the only correctness criterion. Explain expected differences caused by decoder conventions, algorithm choices, or rounding.

Run the repository's applicable formatting, lint, test, and build commands. Record actual results and any checks that could not be run.

## 7. Execution protocol and final deliverables

Work in this order:

1. Start `RES-01` through `RES-08` as concurrent, tracked research tasks. Run
   `RES-09` only after its existing-decoder and Rust ecosystem report PRs are
   merged.
2. Have each owner record findings and evidence in exactly one report under
   `agent-work/research/`.
3. Run `SYN-01` after its prerequisite report PRs are approved and merged, then
   `SYN-02` after `SYN-01`, then `SYN-03` to produce
   `agent-work/research/SYN-03-summary.md`.
4. Review the reports and the synthesis, resolving or documenting disagreements.
5. Produce the API and data structure design report (`DES-001`–`DES-006`),
   citing `RES-*` or `SYN-*` evidence for every material decision.
6. Assemble the persistent design under `docs/raw-pipeline/` and review it with
   the user. No public type or preprocessing API is finalized before `SYN-03`
   is approved and merged in the dependency task PR.
7. Implement `IMPL-CORE-001` first, then execute the decoder, preprocessing, and
   demosaic tracks only after the relevant PR and core dependency are merged.
8. Compose the pipeline module after decoder conformance and preprocessing are complete,
   migrate Python/devtool, retire the legacy architecture, then run QA and
   update documentation.
9. After `QA-006` and every required task PR are merged into
   `refactor/raw-pipeline-v2`, prepare one promotion PR from that integration
   branch to `main`. It must summarize the complete branch, list actual
   workspace/Python validation results, and identify remaining limitations.
   Only the user may merge the promotion PR.

Each research, synthesis, design, and implementation task has its own TOML
manifest and task PR. Update evidence in the owned deliverable and PR. Do not
begin implementation while a required synthesis or design gate is unresolved.

At the end of the research/design phase, provide a concise review summary and identify the tasks that are ready for implementation. Do not treat research or design documents as proof that the implementation exists.

At the end of implementation, report:

- The final architecture and important research findings.
- The API and data model approved and merged through the design task PR.
- How metadata drives preprocessing before demosaicing.
- Task status on `refactor/raw-pipeline-v2`, including blocked and incomplete
  work.
- Which tasks were implemented concurrently.
- Actual test and validation results.
- Unsupported formats, known limitations, and remaining work.
- Whether the integration branch is ready for its final promotion PR to
  `main`.

The goal is a maintainable, Rust-native RAW pipeline whose data semantics are explicit and whose implementation can be developed incrementally by multiple independent contributors.

## Task pause and resume

Support the commands `start <TASK-ID>`, `pause <TASK-ID>`, `resume <TASK-ID>`, and `status <TASK-ID>`.

- `pause`: If the task is running, interrupt it when supported. Before stopping,
  preserve completed findings in its assigned report, keep the task PR open, and
  write a checkpoint under `agent-work/checkpoints/<TASK-ID>/` with unfinished
  work, next steps, unresolved questions, source revisions, and validation
  results.
- `resume`: Read the task definition, owned report or note, latest checkpoint, and reference revisions. Continue unfinished work without repeating verified findings. Verify dependencies before proceeding.
- `status`: Report the task's status from `agent-work/tasks/STATUS.md`, plus
  completed work, remaining work, and blockers.
- `start`: Start a task only if its dependencies are satisfied. Do not duplicate an already-running task.

Persist progress in the task's assigned report and, for pauses or handoffs, a
task-specific checkpoint under `agent-work/checkpoints/`. Keep
`agent-work/tasks/STATUS.md` current, but do not use it as a substitute for
GitHub's review and merge state. Keep the task PR open while paused or
interrupted.
Only the user may approve and merge a task PR.

For delegated tasks, distinguish interrupting a worker from merely asking the coordinator to stop. If the runtime cannot pause a worker directly, explain the limitation and preserve its progress before stopping.
