---
name: demosaic-comparison
description: Add a new reference-versus-Fornax demosaic comparison case, or update an existing one, in crates/compare-demosaic. Use whenever asked to add, port, compare, fix, tune, re-pin, or change a demosaic algorithm, its reference adapter, its waiver, or its comparison note, even if the request does not say "comparison".
---

# Add or update a demosaic comparison

Read [crates/compare-demosaic/AGENTS.md](../../../crates/compare-demosaic/AGENTS.md)
before changing the harness. Use its waiver, note, CLI, and baseline rules
rather than copying them here.

Pick one strategy first.

- **Add**: no case exists yet for this algorithm/reference pair. Follow
  [Strategy A](#strategy-a-add-a-case).
- **Update**: the case exists and its Fornax implementation, reference adapter,
  reference pin, waiver, or hand-written note is changing. Follow
  [Strategy B](#strategy-b-update-a-case). Keep the existing algorithm/reference
  ID; if the change needs a different pair, use Strategy A.

Keep cases table-driven; never add a second comparison framework.

## Strategy A: Add a case

1. Identify the pinned reference algorithm and a stable comparison ID. The
   algorithm/reference pair must be unique.
2. Add the reference adapter at
   `crates/compare-demosaic/cpp/algos/<library>_<algorithm>.{c,cpp}`. Adapt a
   pinned external source without copying its full tree into the comparison
   crate.
3. Wire new C/C++ sources into `crates/compare-demosaic/build.rs`. For a new
   darktable source, also add it to both
   `crates/compare-demosaic/scripts/setup.sh` and
   `crates/compare-demosaic/scripts/setup.ps1`; keep reference versions and
   commit URLs synchronized with `build.rs` and `src/cases.rs`.
4. Add the backend and stable algorithm ID dispatch to
   `crates/compare-demosaic/cpp/shim.cpp` and its header. Add a
   `ReferenceBackend` value only when the comparison uses a new reference
   library.
5. Add the Fornax runner, `Case`, and `CASES` entry in
   `crates/compare-demosaic/src/cases.rs`.
6. Continue with [Check the change](#check-the-change). New rows are never in
   the baseline yet, so an add always ends with one full baseline refresh.

## Strategy B: Update a case

1. List every affected case. Search `CASES` in `src/cases.rs` and the
   `shim.cpp` dispatch for the algorithm, adapter file, or reference being
   changed. A shared adapter, helper, or reference pin can touch several IDs,
   and missing one leaves a stale baseline row.
2. Make the change in the smallest place that owns it: the Fornax code or
   runner, the adapter under `cpp/algos/`, the waiver in `src/waivers.rs`, or
   the note. If a reference pin moves, update `build.rs`, `src/cases.rs`,
   `crates/compare-demosaic/scripts/setup.sh`, and
   `crates/compare-demosaic/scripts/setup.ps1` together.
3. Continue with [Check the change](#check-the-change).

## Check the change

A full `run` computes every row and is the slowest step, and only a full run
can become the baseline. So iterate with partial runs and do the full run once,
last, and only if the result needs to be recorded.

1. Run the affected rows into a scratch directory. `--out` keeps partial output
   away from `target/compare-demosaic/`, so that path only ever holds a full
   run:

   ```bash
   cargo run --release -p compare-demosaic -- run --algorithm <id> \
     [--reference <ref>] [--pattern <P>] --out target/dbg
   ```

2. Compare those rows with the baseline. `diff` compares only the rows a
   partial run selected, so this is the cheap check:

   ```bash
   cargo run --release -p compare-demosaic -- diff \
     crates/compare-demosaic/baseline/canonical.json \
     target/dbg/canonical.json
   ```

3. Decide whether the baseline must change.

   | Situation | Baseline | Full run |
   | --- | --- | --- |
   | Add (new rows) | must change | yes, once |
   | Update, `diff` exits 0 (note, waiver text, refactor, or behavior-neutral change) | unchanged | no |
   | Update, `diff` exits 1 on the selected rows | must change, because CI fails on any difference | yes, once |
   | Shared code touched (`build.rs` flags, common `shim.cpp` paths, shared helpers or metrics, inputs, a pin used by several cases) | likely changes other rows | yes |

   If the numbers move and you did not intend that, investigate with the
   failure order in AGENTS.md before refreshing anything. Never loosen
   `meta.limits`, and never waive a failing row.

4. Run the crate tests; accept only intended snapshots after review:

   ```bash
   cargo insta test -p compare-demosaic
   cargo insta accept
   ```

5. Update the algorithm's hand-written reference sections when its assumptions,
   reference, matches, or known differences change. Use one heading per
   reference, such as `### darktable \`demosaic_ppg\``, followed by Why this
   reference, Why we match, Known differences, and optionally Open issues. Do
   not combine references in one section. Explain causes and conventions, not
   current numbers, and name functions rather than paths. Put any matching
   Accepted outliers paragraph under its reference heading. Do not edit text
   inside generated markers. Update or remove the entry in`src/waivers.rs`
   when the reviewed outliers change; `readme` reports unused waivers.

6. If the table says the baseline must change, refresh it with the one full
   run. Skip this step otherwise.

   ```bash
   cargo run --release -p compare-demosaic -- run
   cp target/compare-demosaic/canonical.json \
     crates/compare-demosaic/baseline/canonical.json
   cargo run --release -p compare-demosaic -- diff \
     crates/compare-demosaic/baseline/canonical.json \
     target/compare-demosaic/canonical.json
   ```

   Never hand-edit the baseline or copy a partial run over it. If local numbers
   differ from CI, take `canonical.json` from the CI artifact instead.

7. Regenerate and verify the README regions (regenerate only when the baseline
   or a note changed):

   ```bash
   cargo run --release -p compare-demosaic -- readme
   cargo run --release -p compare-demosaic -- readme --check
   ```

8. Review `git diff`. For an update, rows outside the affected IDs should not
   have changed; if they did, the cause is shared, so recheck the change rather
   than accepting it.
9. Commit what the change touched in the same pull request: adapter, Rust
   case, waiver, snapshots, baseline, generated README regions, and the
   complete note.
