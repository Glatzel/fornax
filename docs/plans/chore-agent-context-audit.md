# Agent context audit

This note records the Task A inventory, classification, and rule-preservation
map. Byte counts are from branch base `082bfb1`; token estimates use bytes / 4.
This file is on-demand documentation and is not agent session-start context.

## Inventory

| File | Bytes | Est. tokens | Loading class | Classification |
| --- | ---: | ---: | --- | --- |
| `AGENTS.md` | 10,996 | 2,749 | unconditional | mixed must-know, sometimes, redundant |
| `/home/glatzel/.pixi/envs/ai/RTK.md` | 547 | 137 | unconditional; externally managed | must-know tool behavior |
| `.agents/skills/decoder-corpus-tracker/SKILL.md` description | 216 | 54 | unconditional catalog entry | must-know discovery text |
| `.agents/skills/demosaic-comparison/SKILL.md` description | 327 | 82 | unconditional catalog entry | must-know discovery text |
| **Always-loaded total** | **12,086** | **3,022** | | |
| `TODO.md` | 2,207 | 552 | when selecting a bracketed task ID | sometimes; human-owned |
| `docs/architecture/overview.md` | 19,285 | 4,821 | when changing architecture/codec contracts | sometimes |
| `crates/compare-demosaic/AGENTS.md` | 7,549 | 1,887 | nearest-file guide in that crate | sometimes |
| `book/AGENTS.md` | 4,530 | 1,133 | nearest-file guide when editing the book | sometimes |
| `crates/decoder-tracker/AGENTS.md` | 8,690 | 2,173 | nearest-file guide in that crate | sometimes |
| `.agents/skills/decoder-corpus-tracker/SKILL.md` body | 8,115 | 2,029 | when the corpus procedure is invoked | sometimes |
| `.agents/skills/demosaic-comparison/SKILL.md` body | 6,448 | 1,612 | when the demosaic comparison procedure is invoked | sometimes |

All inventoried files total 68,367 bytes (17,092 estimated tokens). That is not
a per-session total. The 40% target applies to the 12,086-byte unconditional
set: at most 4,834 bytes. The externally managed RTK file and skill
descriptions remain unchanged.

## Classification decisions

### Must know in almost every session

- Conflict precedence and the decode/preprocess/demosaic stage boundary.
- Minimal crate layout, facade choice, and the Bayer-only support boundary.
- Branch/worktree/PR convention, TODO ownership, scoped-change rule.
- Validation commands, no local Markdown lint, reporting skipped checks.
- No binaries, root-file layout, project-skill location, provenance rules.
- XERJ's evidentiary rule: external reference search only; verify `file:line`.

### Needed only for specific work

- Complete crate dependency/ownership map and decoder contract:
  `docs/architecture/overview.md`.
- Decoder/codec invariants and benchmark matrix:
  `docs/architecture/decoder.md`,
  `docs/reference/decoders/operations/benchmark.md`.
- Corpus lifecycle, tracker commands, baseline rules:
  `crates/decoder-tracker/AGENTS.md`,
  `docs/reference/decoders/operations/corpus.md`.
- Demosaic comparison commands, baselines, waivers:
  `crates/compare-demosaic/AGENTS.md`, its project skill.
- Book writing and binary-asset rules: `book/AGENTS.md`.
- Detailed branch, validation, PR, and external-data procedure:
  `docs/agent-workflow.md`.

### Removed from always-loaded context as redundant

- The long project map duplicated `docs/architecture/overview.md`.
- The “Where to look next” catalog duplicated document titles already exposed
  by this audit and available through `rg --files`.
- Detailed validation internals moved to `docs/agent-workflow.md`; the command
  names remain in root `AGENTS.md`.
- The work matrix and plan-template checklist moved to
  `docs/agent-workflow.md`.

No rule was intentionally deleted. Content was moved to a more specific
authority or retained in shorter form.

## Quiet tooling findings

- Criterion benchmark probes are the clearest high-volume case.
  `scripts/agent-run.sh` now provides the summary-by-default wrapper and keeps
  the complete log.
- A full `compare-demosaic run` can print one row per case. It already writes
  canonical JSON, so the generic wrapper is enough; changing its established
  text/JSON interface would be a larger compatibility change.
- `decoder-tracker run` already defaults to at most two selected files. Full
  shard runs are manual, and the generic wrapper covers them.
- `scripts/agent-validation.sh` already uses quiet Cargo modes and must preserve
  diagnostics on failure. No command-specific formatter is justified.
- CI helpers keep raw streaming output because workflows consume it and upload
  artifacts; wrapping them would obscure the CI contract.

## Decoder documentation consolidation

The former `docs/decoder/` tree and `docs/reference/decoders/` overlapped
without being byte-identical. Format pages repeated container, codec, metadata,
and acceptance material while reference notes owned source provenance and
detailed behavior.

The documentation now has one decoder tree:

- `docs/reference/decoders/operations/` contains benchmark and corpus runbooks.
- `docs/reference/decoders/reference/` contains support summaries, acceptance
  gates, pinned provenance, and detailed implementation behavior.
- `docs/reference/decoders/reference/dng-decoder.md` consolidates the former
  broad DNG format detail.

`docs/decoder/` is deleted. All inbound links now target the canonical notes or
operations runbooks.

## Old rule -> new location

| Old rule | Where it lives now |
| --- | --- |
| User request wins, then nearest `AGENTS.md` over root | Root `AGENTS.md`, first paragraph |
| Rust RAW workspace with decode/preprocess/demosaic/RGB roles | Root first paragraph; full flow in `docs/architecture/overview.md` |
| Read architecture for crate roles/interfaces/data flow | Root architecture trigger; detailed text retained in `docs/architecture/overview.md` |
| Keep decoding, preprocessing, demosaicing separate | Root first paragraph and architecture overview |
| `fornax-raw-core` ownership/exclusions | `docs/architecture/overview.md`, crate map and decoder contract |
| `fornax-tiff` tag/directory/parser/writer ownership | `docs/architecture/overview.md`, crate map |
| `fornax-codec` request/buffer/Huffman/generic-codec ownership | `docs/architecture/overview.md`, codec API |
| Vendor crates own private enums/streams; no generic feature flags | Root core-layout bullet; full detail in architecture codec API |
| `fornax-dng` tag/directory/transform/write ownership | `docs/architecture/overview.md`, crate map and DNG support |
| Default decoder and `fornax-raw` facade behavior | Root facade bullet; feature detail in architecture overview |
| Rawler/DNC scope | `docs/architecture/overview.md`, crate map |
| Decoder returns mosaic/metadata behind typed interface | `docs/architecture/overview.md`, decoder contract |
| Pipeline owns normalization/demosaic/RGB/postprocessing | Root layout bullet; full detail in architecture pipeline flow |
| Decoder vs pipeline benchmark ownership | `docs/reference/decoders/operations/benchmark.md` and architecture crate map |
| Benchmark matrix uses `--no-default-features` and required feature only | `docs/agent-workflow.md`, Benchmark matrix and validation |
| Tracker ownership and manual-only/full-run rules | `crates/decoder-tracker/AGENTS.md`; dispatch details in `docs/agent-workflow.md` |
| LibRaw/darktable/RawTherapee are dev-only comparison references | Root reference-boundary bullet; usage in `crates/compare-demosaic/AGENTS.md` |
| Active target is conventional 2x2 RGB Bayer; no other CFA claim | Root first paragraph; also architecture and package guides |
| Read `TODO.md` before selecting bracketed task ID | Root Work trigger; file remains human-owned and unchanged |
| Start from `origin/main`; one request per branch/worktree/PR | Root Work; commands and mapping table in `docs/agent-workflow.md` |
| Task/issue/chore branch, worktree, and PR naming table | `docs/agent-workflow.md`, Choosing work |
| Short lowercase kebab-case slug | Root Work |
| TODO changes only for selected completed task or named request | Root Work |
| Fetch/create worktree commands | `docs/agent-workflow.md`, Choosing work |
| Implement only requested change/tests/docs | Root Work |
| Validate, inspect diff, then add/commit/push/create PR | Root Validate and handoff; command block in workflow |
| Open one semantic PR and report URL; include task/issue summary | Root handoff; details in `docs/agent-workflow.md` |
| Plan nontrivial work; goal/scope/checklist/files/validation/decisions | Root trigger; complete rule in `docs/agent-workflow.md` |
| Plan path by task/issue/chore ID | `docs/agent-workflow.md`, Plans |
| Reuse/hard-link main `external/`, never edit shared inputs | Root validation bullet; command/detail in workflow |
| Standard, `--fix`, and `--docs-only` validation commands | Root command block |
| Validation script contents and compare-demosaic trigger | `docs/agent-workflow.md`, Benchmark matrix and validation |
| Run task-specific checks; record outcomes; never claim unrun checks pass | Root Validate |
| Markdown lint is CI-only | Root Validate |
| Semantic PR title; follow-up on same PR; user owns merge/watch/close | Root handoff |
| No binary files, enforced for the book by `check_assets.py` | Root hard boundaries |
| Project skills stay in `.agents/skills`; pixi-managed skills untouched | Root hard boundaries |
| Noisy probes capture full output and print a bounded summary | Root validation bullet and `docs/agent-workflow.md`, Quiet tooling |
| Do not add TODO items; propose them in report/PR | Root Work |
| Root contains only `AGENTS.md` and `TODO.md` as agent files; no tool-specific files | Root hard boundaries |
| Preserve reusable research under `docs/reference/`, not chat/scratch/PR | Root hard boundaries; workflow detail |
| Record pinned sources, behavior, differences, and licence constraints | Root hard boundaries; workflow detail |
| Link implementation/docs to relevant reference notes | `docs/agent-workflow.md`, Reference knowledge |
| Architecture/decoder/benchmark/corpus/plan/reference/book/skill pointers | Root final pointer paragraph; full index in workflow |
| Manual tracker workflow leaves DuckDB tests default and uses path filter | `docs/agent-workflow.md`, final paragraph |
| XERJ only for unfamiliar external code; verify hits; never replaces local evidence | Root hard boundaries |
