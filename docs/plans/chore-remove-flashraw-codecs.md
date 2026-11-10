# Remove `flashraw-codecs` and Own the Codec Paths

## Goal

Remove the production dependency on `flashraw-codecs` by implementing the
currently used Sony and DNG codec behavior in Fornax while preserving bit-exact
sample output for every supported path.

## Scope

In scope:

- Independent in-tree implementations for ARW2, ARW6, and Sony lossless-JPEG
  tile adaptation.
- The format-neutral lossless-JPEG codec remains in `fornax-codec`; Sony codecs
  become public through the `fornax-codec` `arw` feature.
- Independent DNG `OpcodeList1` parsing and pixel transforms in `fornax-dng`.
- Dependency graph, lockfile, architecture, reference, and feature documentation
  updates.
- Synthetic and fixture-backed regression coverage, including exact parity
  checks against pre-migration output.

Out of scope:

- New Sony YCbCr, reduced-resolution, ARQ, or multi-component RAW support.
- New geometric DNG opcode support or `OpcodeList2`/`OpcodeList3` application.
- Changes to decoder contracts, raw pipeline semantics, or `TODO.md`.
- Copying or adapting implementation source from `flashraw-codecs`, Rawler,
  LibRaw, or other reference projects.

## Task Checklist

- [ ] Create the branch worktree and record baseline test results.
- [ ] Capture pre-migration raw-sample SHA-256 hashes for the supported Sony
  fixtures and a DNG fixture containing `OpcodeList1`.
- [ ] Add independent DNG opcode parsing and application with unit tests for
  every accepted pixel operation and rejection path.
- [ ] Add the `arw` feature and Sony codec modules to `fornax-codec`; move ARW2
  curve handling behind the shared codec API.
- [ ] Add Sony lossless-JPEG tile adaptation using the owned lossless-JPEG
  decoder.
- [ ] Implement ARW6 stream parsing, coefficient decoding, inverse lifting,
  color reconstruction, Bayer expansion, and the LLVC3 output curve.
- [ ] Route `fornax-arw` through `fornax-codec`, preserve its format-layer
  metadata behavior, and remove all `flashraw-codecs` imports.
- [ ] Remove the workspace dependency and obsolete transitive lock entries.
- [ ] Update active architecture, codec feature/provenance, Sony, and DNG
  opcode reference documentation.
- [ ] Run feature-matrix, fixture-backed, workspace, and dependency-graph
  validation; inspect the final diff and open the pull request.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`
- `crates/fornax-codec/Cargo.toml`, `crates/fornax-codec/src/{lib.rs,encode.rs,request.rs}`
- `crates/fornax-codec/src/arw/{mod.rs,arw2.rs,arw6.rs,ljpeg.rs}`
- `crates/fornax-arw/Cargo.toml`, `crates/fornax-arw/src/{decode.rs,metadata.rs}`
- `crates/fornax-dng/Cargo.toml`,
  `crates/fornax-dng/src/dng_decoder/{mod.rs,transforms.rs,opcodes.rs}`
- `docs/architecture.md`, `docs/references/{sony-arw.md,dng-lossless-jpeg.md}`
- `docs/references/dng-opcodes.md`
- `crates/fornax-codec/README.md`, `scripts/agent-validation.sh`
- Focused test modules under the affected crates

## Validation

- `cargo check -p fornax-codec --no-default-features`
- `cargo test -p fornax-codec --features lossless-jpeg`
- `cargo test -p fornax-codec --features arw`
- `cargo clippy -p fornax-codec --all-features`
- `cargo test -p fornax-dng`
- Fixture-backed `cargo test -p fornax-arw` with all supported
  `FORNAX_SONY_*_FIXTURE` variables set.
- `scripts/agent-validation.sh`
- `cargo tree --workspace | rg flashraw` must produce no matches.
- `git diff --check`

## Decisions and Notes

- Exact parity is required for ARW1, ARW2, ARW6, four-component Sony LJPEG, and
  currently accepted `OpcodeList1` operations.
- All Sony codecs live in `fornax-codec` under the non-default `arw` feature.
  `fornax-arw` retains format parsing and metadata ownership only.
- DNG opcodes remain in `fornax-dng` because they are metadata-driven pixel
  transforms, not compression codecs.
- `flashraw-codecs` is a temporary black-box behavioral oracle only. It must
  not be present in committed production or development dependency lists after
  the migration.
- The fixed Sony LLVC3 curve is treated as Sony format data. Its provenance
  must be recorded before commit.
- Existing dirty RAW corpus payloads are local validation inputs and must not be
  committed or rewritten.
