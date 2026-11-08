# Agent Guidance Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update agent-facing instructions, architecture notes, and decoder
guides to match the decoder and pipeline crates in PR 646.

**Architecture:** `fornax-core` owns shared types and traits. Dedicated DNG,
Rawler, and DNC decoder crates implement `IDecoder`; preprocessing and
demosaicing remain separate stage crates. `bench-decoder` owns decoder
benchmarks, while demosaic and preprocessor benchmarks stay with their stage
crates.

**Tech Stack:** Rust 2024 workspace, GitHub Actions, Markdown instructions,
and project-local agent skills.

**Spec:** The user request to update every agent-related file after the broad
PR 646 code changes. Do not run the heavy compare-demosaic generation command;
therefore leave generated README data and conclusions unchanged.

## Global Constraints

- Keep decoding, preprocessing, and demosaicing as separate stages.
- Do not claim support beyond conventional 2x2 RGB Bayer in the active
  preprocessor/demosaic pipeline.
- Do not run `compare-demosaic run`, `readme`, or baseline refresh commands.
- Do not edit generated compare-demosaic README regions or accepted results.
- Preserve LibRaw, darktable, and RawTherapee as comparison-only references.
- Update historical plans only when they remain active instructions; do not
  rewrite completed plans as current architecture.
- Run only documentation-focused checks plus the cheap benchmark target
  compilation needed to confirm the documented command.

## Review Focus

- Root guidance must name the default DNG decoder and optional Rawler/DNC
  adapters without describing them as removed.
- The `IDecoder` lifecycle must be described as decode, then raw/cropped mosaic
  and DNG/process metadata access on the returned decoder instance.
- Validation guidance must distinguish the root workspace checks from the
  separate compare-demosaic package job.
- Compare-demosaic setup paths must match the scripts moved under the crate.
- Project-skill commands must remain executable from the documented working
  directory.

## Task Checklist

- [x] Inventory agent instruction files, project skills, referenced
      architecture documents, crate manifests, decoder APIs, CI, and setup paths.
- [x] Confirm the latest PR head and leave `book/AGENTS.md` unchanged because
      its book-specific rules still match the repository.
- [x] Update root `AGENTS.md` crate map, decoder invariants, validation, and
      reference links.
- [x] Update workspace and decoder architecture documents for the new decoder
      crates, image wrappers, metadata types, and pipeline data flow.
- [x] Update decoder benchmark and corpus guidance for `bench-decoder` and the
      current `IDecoder` lifecycle while preserving the open fixed-corpus work.
- [x] Update the DNG and other format notes where they still name removed
      core decoder types.
- [x] Update compare-demosaic agent and skill setup/CI guidance without running
      comparison generation or editing result data.
- [x] Validate links, stale API names, Markdown formatting, YAML, skill
      frontmatter, and `bench-decoder` target compilation.
- [x] Record outcomes, inspect the diff, commit, and push to PR 646.

## Files Expected to Change

- `AGENTS.md`
- `crates/compare-demosaic/AGENTS.md`
- `.agents/skills/demosaic-comparison/SKILL.md`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/benchmark.md`
- `docs/decoder/corpus.md`
- `docs/decoder/formats/canon-cr3.md`
- `docs/decoder/formats/dng.md`
- `docs/decoder/formats/fuji-raf.md`
- `docs/decoder/formats/hasselblad-3fr.md`
- `docs/decoder/formats/panasonic-rw2.md`
- `docs/plans/chore-agent-guidance-refresh.md`

## Validation

```bash
git diff --check
cargo bench -p bench-decoder --no-run
yq eval '.' .github/workflows/ci.yml >/dev/null
```

Also:

- Check every relative Markdown link in changed instruction/reference files.
- Search changed guidance for removed API terms:
  `fornax_core::Decoder`, `decode_active`, `DecodedRaw`, `MosaicData`,
  `RawImage<T>`, and `Metadata<T>`.
- Validate project-skill frontmatter with the skill validator.
- Confirm no compare-demosaic baseline, generated README region, or result
  snapshot changed.
- Record the intentionally skipped heavy comparison command in the final
  report.

## Decisions and Notes

- The active `IDecoder` trait takes an input, returns a decoder instance, then
  exposes `raw_mosaic`, `cropped_mosaic`, `dng_metadata`, and
  `process_metadata`.
- `fornax-decoder` is the default pure-Rust DNG decoder.
  `fornax-decoder-rawler` and Windows/macOS-gated `fornax-decoder-dnc` are
  alternate decoder adapters available through the facade.
- `ProcessMetadata<T>` replaces pipeline `Metadata<T>` and stores
  green-normalized white-balance coefficients directly.
- `Decoder<T>` is generic only because `cropped_mosaic` and process metadata
  target the requested primitive type.
- `book/AGENTS.md` needs no factual change; the user asked not to do unrelated
  book work.
- The compare-demosaic README may still be accurate outside generated result
  regions, but this task does not regenerate or revise results.
- `git diff --check`, the changed-file link check, YAML parsing, and
  `cargo bench -p bench-decoder --no-run` passed.
- The project-skill frontmatter was parsed successfully with `yq`. The
  skill-creator Python validator could not start because PyYAML is not
  installed in the active interpreter.
- No Markdown-lint executable is installed in the active environment, so that
  check was skipped.
- The full Rust suite and every compare-demosaic generation/baseline command
  were intentionally not run for this documentation-only task.
