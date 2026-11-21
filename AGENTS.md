# Fornax agent rules

User request wins, then the closest `AGENTS.md`, then this file. Fornax is a
Rust RAW pipeline; keep decode, preprocess, demosaic, and postprocess separate.
Support conventional 2x2 RGB Bayer only; do not claim X-Trans or other CFAs.

- Core layout: `fornax-raw-core` owns shared image/stage contracts;
  `fornax-tiff` generic TIFF; `fornax-codec` format-neutral codecs;
  `fornax-dng` DNG; `fornax-raw-*` private vendor parsing/codecs;
  `fornax-decoder` facade; `fornax-raw-pipeline` processing; `fornax-raw`
  public facade. Vendor stream behavior never moves behind a codec feature.
- `fornax-decoder::Decoder` is the default pure-Rust facade. DNG is
  unconditional; seven vendor features are enabled by default. Read
  `docs/architecture/overview.md` before changing crate boundaries or codec
  contracts.
- LibRaw, darktable, and RawTherapee are development-only research references,
  never production decoder dependencies.

## Work

- `TODO.md` is human-owned. Read it only before selecting a bracketed ID; never
  add items—propose them in the report/PR. Remove a selected item only when its
  task is complete.
- Start from current `origin/main`, one request per branch/worktree/PR:
  `task/<id>-<slug>`, `fix/<issue>-<slug>`, or `chore/<slug>`. Use a short
  kebab-case slug. Keep follow-up changes on the same PR.
- Read `docs/agent-workflow.md` before branching, writing a plan, changing
  validation, or preparing a PR. Nontrivial work needs a `docs/plans/` note.
- Implement only the request and its required tests/docs. Inspect the diff;
  report failed or skipped checks exactly.

## Validate and hand off

```bash
scripts/agent-validation.sh
scripts/agent-validation.sh --fix
scripts/agent-validation.sh --docs-only
```

- Use `--docs-only` for docs/context-only changes. Never run Markdown lint
  locally. Also run task-specific checks and record commands and outcomes.
- For noisy probes, use `scripts/agent-run.sh -- <command>`; it preserves the
  status and writes full output under `target/agent-logs/`.
- In a linked worktree, hard-link the main checkout's reusable `external/`
  fixture data with `rsync --link-dest`; initialize the pointer-only
  raw.pixls.us submodule in that worktree. Treat hard-linked files as read-only.
- Commit, push, and open one semantic PR using a title such as
  `docs: rewrite agent guidance`. Only the user merges or abandons a PR; do not
  poll, watch, or close it or a linked issue.

## Hard boundaries

- Commit no binary files. In `book/`, CI enforces this with
  `book/scripts/check_assets.py`.
- Project procedures live in `.agents/skills/`; change them by PR. Do not add
  `CLAUDE.md` or other tool-specific root instruction files. The only agent
  files at root are `AGENTS.md` and `TODO.md`. Never hand-edit pixi-managed
  skills.
- Preserve reusable algorithm/source research under `docs/reference/`, with
  pinned provenance and licence constraints. Do not leave findings only in
  chat or PR comments.
- Use XERJ only for unfamiliar external reference code. Verify every hit at
  `file:line`; it never replaces reading `TODO.md`, `AGENTS.md`, generated
  README tables, Cargo output, or diffs.

For task procedures, read `docs/agent-workflow.md`; for decoder benchmarks and
corpus work, read `docs/architecture/decoder.md`,
`docs/reference/decoders/operations/benchmark.md`, and
`docs/reference/decoders/operations/corpus.md`; for the human-facing book, read
`book/AGENTS.md` only when asked to edit it.
