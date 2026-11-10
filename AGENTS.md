# Agent instructions — fornax

## Overview

Fornax is a Rust workspace for decoding RAW images, normalizing Bayer mosaics,
and reconstructing RGB output through separate pipeline stages. When
instructions conflict, the user's explicit request wins. Otherwise, the
`AGENTS.md` closest to the files being changed wins over this root file.

## Project map

Read [docs/architecture.md](docs/architecture.md) for crate roles, interfaces,
and pipeline data flow. These invariants always apply:

- Keep decoding, preprocessing, and demosaicing as separate stages.
- `fornax-raw-core` owns shared image wrappers, process metadata, errors, and stage
  traits. It does not own TIFF/DNG parsing, decoded directory metadata, DNG
  writing, or a decoder implementation.
- `fornax-tiff` owns generic TIFF tags (`Tag` and `TiffTag`), typed TIFF field
  constants, `Directory` values, parsing, and the reusable directory writer.
- `fornax-codec` owns format-neutral raw sample decoding, chunk assembly,
  predictors, and supported codec encoding.
- `fornax-dng` owns DNG-only tags (`DngTag`), a `Directory` keyed by `Tag`,
  DNG-specific transforms, and DNG writing.
- `fornax-decoder::Decoder<T>` is the default pure-Rust DNG decoder. The
  `fornax-raw` facade re-exports it as `fornax_raw::decoder::Decoder` and the
  DNG format layer as `fornax_raw::dng`.
- `fornax-decoder-rawler` is an alternate Rawler-backed decoder for supported
  conventional 2x2 RGB Bayer files. `fornax-decoder-dnc` converts with Adobe
  DNG Converter before decoding with Rawler; its facade export is limited to
  Windows and macOS and it also requires that external converter.
- Decoder implementations return a decoder value from `IDecoder::decode`; the
  returned value exposes `raw_mosaic`, `cropped_mosaic`, `dng_metadata`, and
  `process_metadata`. `dng_metadata` returns a `fornax_tiff::Directory`;
  decoder-specific parsing state must stay behind that interface.
- `fornax-raw-pipeline` owns black/white normalization, demosaic protocols and
  algorithms, RGB reconstruction, and postprocessing.
- `fornax-decoder` owns decoder benchmarks. Demosaic and preprocessor
  benchmarks live in `fornax-raw-pipeline`.
- `decoder-tracker` owns the manual raw.pixls.us decoder regression tracker,
  pointer-only corpus manifest, sharded CI workflow, and text baseline.
- LibRaw, darktable, and RawTherapee remain development-only references for
  `compare-demosaic`; do not add them to production decoder dependencies.
- The active demosaic target is conventional 2x2 RGB Bayer input. Do not claim
  support for X-Trans, four-color, monochrome, or other CFA layouts.

## Choosing work

Read `TODO.md`, the human-owned backlog and completion record, before selecting
a bracketed task ID. Start new requests from the current `origin/main`; keep
requested changes to an open pull request on that pull request's branch and
worktree. One new request maps to one branch, one worktree, and one pull
request.

| Request                                  | Branch                       | Worktree                           | Pull request requirements                                                                                                        |
| ---------------------------------------- | ---------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `TODO.md` item selected by bracketed ID  | `task/<lowercase-id>-<slug>` | `.worktrees/<lowercase-id>-<slug>` | Include the task ID and remove the item from `TODO.md` in the same pull request. Keep the item if work is incomplete or blocked. |
| GitHub issue number supplied by the user | `fix/<number>-<slug>`        | `.worktrees/fix-<number>-<slug>`   | Read only that issue with `gh issue view <number>`, add regression coverage, and include `Fixes #<number>` in the body.          |
| Explicit request not in `TODO.md`        | `chore/<slug>`               | `.worktrees/chore-<slug>`          | Leave `TODO.md` untouched unless the request names an item.                                                                      |

Use a short lowercase kebab-case slug.
Modify `TODO.md` only for a completed selected task, or when the request names an item.

## Workflow

Fetch and create the worktree from `origin/main`:

```bash
git fetch origin
git worktree add -b "<branch>" ".worktrees/<worktree>" origin/main
```

Implement only the requested change and its required tests or documentation.
Run validation, then inspect the diff before committing:

```bash
git add <paths>
git commit -m "<type>: <summary>"
git push -u origin HEAD
gh pr create --base main --title "<type>: <summary>" --body "<body>"
```

Open one semantic pull request and report its URL. Put the task ID, issue
reference, or maintenance summary in the body as applicable.

## Plans

Write a plan only for non-trivial work: several files or crates, a design
decision, or more than a short session. Small items go straight to
implementation.

Store plans under `docs/plans/` using the branch ID and slug:

- Task: `docs/plans/<lowercase-id>-<slug>.md`
- Issue: `docs/plans/fix-<number>-<slug>.md`
- Chore: `docs/plans/chore-<slug>.md`

Commit the plan in the same pull request. Update its progress and record
decisions while the work proceeds; stop treating it as authoritative once the
pull request merges.

Every plan must contain Goal, Scope (in and out), Task checklist, Files
expected to change, Validation, and Decisions and notes. Derive the detailed
checklist from the selected `TODO.md` item and the code without expanding
`TODO.md`. Make the plan self-contained: a reader with only the working tree
and that file must be able to carry out the work.

## Validation

For Rust changes, run workspace validation from the repository root:

```bash
scripts/agent-validation.sh
```

The script initializes `external/raw-images` with `scripts/setup.sh` when its
test fixture is missing, initializes the pointer-only raw.pixls.us submodule
when needed, verifies the corpus manifest, then runs workspace tests, nightly
formatting, documentation, clippy, decoder benchmark compilation, and diff
checks. It also runs `compare-demosaic` package checks when that crate changed.
Its baseline and generated README data have separate rules in
[crates/compare-demosaic/AGENTS.md](crates/compare-demosaic/AGENTS.md).

Pass `--fix` to apply nightly rustfmt and machine-applicable clippy fixes before
running the normal checks:

```bash
scripts/agent-validation.sh --fix
```

For documentation-only changes, run:

```bash
scripts/agent-validation.sh --docs-only
```

Markdown lint runs in CI and must not be run locally by agents.

Also run task-specific tests, benchmarks, or examples. Record commands and
outcomes in the pull request. Report failures and skipped checks explicitly;
never describe an unrun check as passing.

## Pull request handoff

Titles must satisfy `.github/semantic.yml`; for example, use
`docs: rewrite agent guidance`. Put requested changes on the same branch and
pull request. Do not watch, poll, merge, or manually close a pull request or
linked issue. Only the user merges or abandons a pull request.

## Boundaries

- Do not commit binary files to the book; CI enforces this through
  `book/scripts/check_assets.py`.
- Project skills live in `.agents/skills/`. Add or improve them by pull
  request. Skills installed by pixi are managed by pixi; never edit them by
  hand.
- Do not add items to `TODO.md`; propose them in the report or pull request
  description.
- Keep the repository root clean: the only agent-related root files are
  `AGENTS.md` and `TODO.md`. Do not add tool-specific instruction files for a
  particular coding agent.

## Reference knowledge

- Preserve reusable algorithm and implementation research in
  `docs/references/<topic>.md` and commit it with the code it informs. Do not
  leave important findings only in chat, scratch files, or pull request
  comments.
- Record authoritative source links with pinned versions or commits, the
  behavior or decision they support, known implementation differences, and
  license constraints that affect reuse.
- Link implementation modules or generated documentation to the relevant
  reference note when the source material clarifies invariants that are easy
  to regress.

## Where to look next

- Architecture and pipeline boundaries: [docs/architecture.md](docs/architecture.md)
- Decoder ownership and validation: [docs/decoder/architecture.md](docs/decoder/architecture.md)
- Decoder benchmarks and corpus plans: [docs/decoder/benchmark.md](docs/decoder/benchmark.md),
  [docs/decoder/corpus.md](docs/decoder/corpus.md)
- Manual corpus tracker: `cargo run -p decoder-tracker -- --help`; full runs are
  manual-only and RAW payloads are never committed or uploaded.
- When dispatching the `Decoder corpus` workflow, leave
  `run_duckdb_tests` disabled at its default unless explicitly requested, and
  set `formats` to match the requested corpus filter.
- Per-work plans: [docs/plans/](docs/plans/)
- Algorithm and reference notes: [docs/references/](docs/references/)
- Comparison-crate rules:
  [crates/compare-demosaic/AGENTS.md](crates/compare-demosaic/AGENTS.md)
- Human-facing book: [book/](book/README.md); agents edit it only when asked
  and follow [book/AGENTS.md](book/AGENTS.md).
- Project procedures: [.agents/skills/](.agents/skills/)
- Use XERJ only to locate unfamiliar external reference code; verify hits at
  `file:line`. Never substitute it for reading `TODO.md`, `AGENTS.md`, generated
  README tables, Cargo output, or diffs.
