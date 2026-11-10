# Compression ID Ownership Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish source-backed ownership for every requested TIFF-family compression ID, then move only non-DNG IDs to the TIFF vocabulary when the evidence is unambiguous.

**Architecture:** The PR starts as an audit. If every ID has a clear owner, `fornax-tiff` becomes the vocabulary for TIFF 6.0, TIFF technical-note, LibTIFF/GDAL, and other TIFF-family IDs, while `fornax-dng` keeps only DNG-defined IDs. The TIFF resolver distinguishes unknown IDs from known IDs whose codec feature is disabled.

**Tech Stack:** Rust 2024, Cargo features, TIFF/DNG specifications and registry sources.

**Spec:** `docs/plans/workspace-refactor/phase-b-c-handoff.md`

## Global Constraints

- Use branch `chore/compression-id-audit` and worktree
  `.worktrees/chore-compression-id-audit`.
- Use one worktree and one pull request.
- Keep the branch plan at `docs/plans/chore-compression-id-audit.md`.
- Paraphrase specifications; do not paste long normative text.
- Keep `34892` and `52546` in `fornax-dng`.
- DNG decoding results must not change.
- Do not add LZMA, JPEG, WebP, or vendor codec implementations in this step.
- Do not change crate names, project versions, the workspace split, vendor codec placement, ARW/DNG ownership, or the Python API.
- Do not compile `compare-demosaic`.

## Review Focus

- An ID defined only by LibTIFF/GDAL must not remain a DNG-owned enum variant.
- A known ID whose codec feature is off must produce a typed error naming the feature.
- An unknown ID must remain distinguishable from a feature-disabled ID.
- DNG `34892` and `52546` must still resolve to their existing codecs.
- Existing DNG and ARW golden outputs must remain unchanged.

---

### Task 1: Produce the ownership table and stop on ambiguity

**Files:**
- Create: `docs/references/compression-ids.md`
- Modify: `docs/references/dng-compression.md`

**Interfaces:**
- Consumes: the requested IDs `1`, `5`, `7`, `8`, `32773`, `32946`, `34892`, `34925`, `50000`, `50001`, `50002`, and `52546`.
- Produces: an ownership decision for each ID before code changes.

- [ ] **Step 1: Check primary sources**

For each ID, inspect TIFF 6.0, the relevant TIFF technical note, TIFF/EP,
the DNG specification and version, LibTIFF `tiff.h` at a pinned version, and
GDAL documentation where applicable. Record exact document versions or pinned
commit URLs.

- [ ] **Step 2: Write the table**

Create `docs/references/compression-ids.md` with this schema:

```markdown
| ID | Meaning | Defining source and version | DNG-defined? | Fornax owner |
| ---: | --- | --- | --- | --- |
```

Include one row for every requested ID. The table must distinguish:

- normative TIFF-family registration;
- Adobe DNG definition;
- LibTIFF/GDAL self-assignment;
- vendor-private meaning;
- an origin that cannot be established.

Record known implementation limitations separately; do not treat a codec gap
as evidence of ownership.

- [ ] **Step 3: Apply the ambiguity gate**

If any origin is contradictory or cannot be established, stop before changing
production code. Report the uncertain ID with its conflicting sources and ask
the maintainer for a ruling.

- [ ] **Step 4: Update the existing DNG compression reference**

Revise `docs/references/dng-compression.md` so its decisions and ownership
table agree with the new audit. Preserve the rule that `50000` and `50002`
must be described according to their actual external ownership, not as
Adobe-defined DNG values.

- [ ] **Step 5: Commit the audit**

```bash
git add docs/references/compression-ids.md docs/references/dng-compression.md
git commit -m "docs: audit TIFF compression id ownership"
```

### Task 2: Move confirmed non-DNG IDs to TIFF vocabulary

**Files:**
- Modify: `crates/fornax-tiff/src/constants.rs`
- Modify: `crates/fornax-tiff/src/raster.rs`
- Modify: `crates/fornax-tiff/src/lib.rs`
- Modify: `crates/fornax-tiff/Cargo.toml`
- Modify: `crates/fornax-tiff/tests/constants.rs`
- Modify: `crates/fornax-tiff/tests/raster.rs`
- Modify: `crates/fornax-dng/src/constants.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-dng/src/writer.rs`
- Modify: `crates/fornax-dng/Cargo.toml`
- Modify: `crates/fornax-tiff/README.md`
- Modify: `crates/fornax-dng/README.md`
- Modify: `docs/architecture.md`

**Interfaces:**
- Consumes: the Task 1 ownership decisions.
- Produces:

```rust
pub enum TiffCompressionError {
    UnknownCompression(u16),
    UnsupportedCompression {
        id: u16,
        required_feature: &'static str,
    },
}

pub fn resolve_tiff_compression(
    id: u16,
) -> Result<DecodeCompression, TiffCompressionError>;
```

- [ ] **Step 1: Add failing resolver tests**

In `crates/fornax-tiff/tests/raster.rs`, cover:

- `1`, `7`, and `8` resolve with the raster feature;
- `32946` resolves to `DecodeCompression::Deflate`;
- every enabled feature-specific ID resolves to its intended codec;
- a known ID with its feature off returns `UnsupportedCompression` and names
  the feature;
- an unregistered ID returns `UnknownCompression`.

Run:

```bash
cargo test -p fornax-tiff --all-features --test raster
cargo test -p fornax-tiff --no-default-features --features raster --test raster
```

Expected: FAIL because the result type and variants do not exist.

- [ ] **Step 2: Expand TIFF compression vocabulary**

Add confirmed TIFF-family IDs to `fornax_tiff::constants::CompressionMethod`.
Keep one `Unknown(u16)` catch-all. Add Cargo features forwarding the matching
codec capabilities:

```toml
jpegxl = ["raster", "fornax-codec/jpegxl"]
zstd = ["raster", "fornax-codec/zstd"]
```

Use unconditional `UnsupportedCompression` results for confirmed IDs whose
codec features are not yet implemented.

- [ ] **Step 3: Remove only non-DNG variants**

Remove all IDs confirmed non-DNG from `DngCompressionMethod`. Keep DNG lossy
JPEG `34892` and DNG JPEG XL `52546`. Rename the retained DNG JPEG XL variant
to `JpegXl` if the source table confirms that no legacy TIFF value belongs in
the DNG enum.

- [ ] **Step 4: Route DNG through the TIFF resolver**

In `fornax-dng`, resolve only its retained DNG-owned IDs first, then call
`resolve_tiff_compression`. Convert `TiffCompressionError` into the existing
typed DNG decode error without changing supported outputs.

Enable `fornax-tiff/jpegxl` in the DNG dependency. Change DNG's `zstd` feature
to forward both `fornax-codec/zstd` and `fornax-tiff/zstd`.

- [ ] **Step 5: Update constant and writer tests**

Update `crates/fornax-tiff/tests/constants.rs` to round-trip every confirmed
TIFF-owned ID. Update DNG tests so the DNG enum rejects every non-DNG ID and
retains exactly `34892` and `52546` as named values. Update writer tests so
Zstandard writes `CompressionMethod::Zstd`.

- [ ] **Step 6: Run focused tests**

```bash
cargo test -p fornax-tiff --no-default-features
cargo test -p fornax-tiff --all-features
cargo test -p fornax-dng --all-features
cargo test -p fornax-arw
```

Expected: all commands exit 0.

- [ ] **Step 7: Commit implementation**

```bash
git add crates/fornax-tiff crates/fornax-dng docs/architecture.md
git commit -m "refactor: separate DNG and TIFF compression ownership"
```

### Task 3: Validate the ownership split

**Files:**
- No additional source files.

**Interfaces:**
- Consumes: Tasks 1 and 2.
- Produces: PR validation evidence.

- [ ] **Step 1: Run workspace validation**

```bash
scripts/agent-validation.sh
```

Expected: exit 0.

- [ ] **Step 2: Check dependency trees**

```bash
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1 --features raster,jpegxl,zstd
```

Expected: neither crate contains raw workflow types.

- [ ] **Step 3: Re-run golden hashes**

```bash
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: `golden: unchanged (9 rows)`.

- [ ] **Step 4: Commit validation**

Update the branch plan with commands, outcomes, and the completed ownership
table, then commit that documentation.
