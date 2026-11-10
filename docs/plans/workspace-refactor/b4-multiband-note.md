# Multi-Band Container Design Note Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record evidence and implementation consequences for future multi-band and wider-sample support without selecting a container shape.

**Architecture:** This is a documentation-only decision-support PR. It compares additive `SampleBuffer` variants with a separate raster/container API and records how each option would affect codec requests, DNG adapters, public compatibility, and raw-workflow separation.

**Tech Stack:** Rust API documentation and Cargo feature analysis.

**Spec:** `docs/plans/workspace-refactor/phase-b-c-handoff.md`

## Global Constraints

- Use branch `chore/multiband-note` and worktree
  `.worktrees/chore-multiband-note`.
- Use one worktree and one pull request.
- Keep the branch plan at `docs/plans/chore-multiband-note.md`.
- Do not change production code, versions, Cargo features, or dependencies.
- Do not choose a multi-band container shape.
- Do not choose the representation of `i16` or `f64`.
- Keep raw workflow types out of `fornax-codec` and `fornax-tiff`.
- Do not compile `compare-demosaic`.

## Review Focus

- The note must distinguish current single-band `SampleBuffer` behavior from hypothetical multi-band behavior.
- `i16`/`f64` storage must not be conflated with pipeline `Primitive` conversion.
- The DNG path's current mono adapter and unsupported multi-channel `LinearRaw` cases must be explicit.
- Public enum compatibility and the `image` 0.x dependency constraint must be included.
- The note must end with options and consequences, not a recommendation or selected shape.

---

### Task 1: Create the options note

**Files:**
- Create: `docs/plans/workspace-refactor/multiband-note.md`
- Modify: `docs/plans/workspace-refactor/README.md`

**Interfaces:**
- Consumes: implemented `SampleBuffer`, codec request, TIFF raster, and DNG adapter contracts.
- Produces: an evidence table and option analysis for the reserved container decision.

- [ ] **Step 1: Record current behavior**

Document with repository-relative references:

- `SampleBuffer` is single-band and supports only U8, U16, U32, and F32;
- each public variant wraps `image::ImageBuffer<Luma<T>, Vec<T>>`;
- `image` 0.x upgrades can break users of the public variants;
- codec requests carry one plane;
- `fornax-dng` privately converts between `SampleBuffer` and mono
  `RawMosaic`;
- DNG multi-channel `LinearRaw` is currently unsupported.

- [ ] **Step 2: Analyze additive variants**

Describe an option where the existing `SampleBuffer` enum gains one or more
variants. Cover:

- effect on source compatibility and exhaustive matching;
- effect on `width`, `height`, raw accessors, and sample extraction;
- effect on `DecodeRequest`, `EncodeRequest`, and predictor geometry;
- effect on TIFF strip/tile layout;
- effect on the DNG mono adapter;
- effect on public `image` type exposure;
- interaction with `i16` and `f64`.

Do not define variant names, band counts, or storage layouts.

- [ ] **Step 3: Analyze a separate container**

Describe an option where samples live in a new multi-band/raster container
used by codecs and format crates. Cover the same impacts, plus:

- whether the adapter grows into a general conversion layer;
- compatibility between mono and multi-band call paths;
- how raw-workflow separation is preserved;
- how TIFF consumers avoid depending on DNG/raw-core types.

Do not define the container type, bounds, lifetime, or public signatures.

- [ ] **Step 4: Add evidence and open questions**

Include a comparison table with columns:

```markdown
| Question | Additive variants | Separate container | Evidence |
| --- | --- | --- | --- |
```

End with unresolved questions and a direct link to the Phase B-C handoff's
reserved decisions. Do not write "recommended", "preferred", or a selected
shape.

- [ ] **Step 5: Link the note**

Add the new note to `docs/plans/workspace-refactor/README.md`.

- [ ] **Step 6: Validate documentation**

```bash
scripts/agent-validation.sh --docs-only
git diff --check
```

Expected: both commands exit 0, and local Markdown lint is not run.

- [ ] **Step 7: Re-run required global checks**

```bash
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: dependency trees are unchanged and
`golden: unchanged (9 rows)`.

- [ ] **Step 8: Commit**

```bash
git add docs/plans/workspace-refactor/multiband-note.md \
  docs/plans/workspace-refactor/README.md
git commit -m "docs: record multiband container options"
```
