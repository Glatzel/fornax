# User Review Gate Checkpoint — 2026-10-23

> Historical workflow checkpoint. The former repository-local review gate has
> been replaced by task pull requests; status wording below is preserved as
> history.

**Reason:** The user requires review of completed work and may request changes
or extra work before accepting any task as `done`.

## Ready for user review

| Task | Status | Report |
| --- | --- | --- |
| `RES-01` | `review` | `agent-work/research/RES-01-fornax-inventory.md` |
| `RES-02` | `review` | `agent-work/research/RES-02-libraw.md` |
| `RES-03` | `review` | `agent-work/research/RES-03-dnglab.md` |
| `RES-04` | `review` | `agent-work/research/RES-04-darktable.md` |
| `RES-05` | `review` | `agent-work/research/RES-05-demosaicing.md` |
| `RES-06` | `review` | `agent-work/research/RES-06-rust-raw-decoders.md` |
| `RES-07` | `review` | `agent-work/research/RES-07-rust-dng-tiff.md` |
| `RES-08` | `review` | `agent-work/research/RES-08-rust-demosaicing.md` |

Primary checks are complete for all eight reports:

- `agent-work/research/RES-07-rust-dng-tiff.md` distinguishes generic TIFF support from
  the missing DNG/CFA metadata model.
- `agent-work/research/RES-08-rust-demosaicing.md` records Bayer API, border,
  validation, allocation, registry, and synthetic-test behavior while excluding
  non-target sensor layouts.

`RES-09` remains `blocked` until the user approves RES-06–RES-08 as `done`.

## Validation completed

- `cargo test -p fornax-dalim --all-features`: 4 passed, 0 failed.
- `git diff --check` passes.
- All research submodules remain clean and pinned.
- RES-03 no longer cites the missing extracted DNG text file.

## Next steps

1. User reviews RES-01–RES-08 and requests any corrections or extra work.
2. Move approved reports to `done`.
3. Start RES-09 only after RES-06–RES-08 are user-approved as `done`.
4. Do not start synthesis, design, or implementation without a new user request.

## Working tree

Research and tracker updates are uncommitted. Preserve them when resuming.
