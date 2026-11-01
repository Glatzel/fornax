---
name: add-demosaic-comparison
description: Add a reference-versus-Fornax demosaic comparison case to crates/compare-demosaic; use when asked to add, port, or compare a demosaic algorithm against a reference implementation.
---

# Add a demosaic comparison

Read [crates/compare-demosaic/AGENTS.md](../../../crates/compare-demosaic/AGENTS.md)
before changing the harness. Use its waiver and CLI rules rather than copying
them here.

1. Identify the pinned reference algorithm and a stable comparison ID. Keep
   cases table-driven; do not add a second comparison framework.
2. Add the reference adapter at
   `crates/compare-demosaic/cpp/algos/<library>_<algorithm>.{c,cpp}`. Adapt a
   pinned external source without copying its full tree into the comparison
   crate.
3. Wire new C/C++ sources into `crates/compare-demosaic/build.rs`. For a new
   darktable source, also add it to both `scripts/setup.sh` and
   `scripts/setup.ps1`; keep reference versions and commit URLs synchronized
   with `build.rs`.
4. Add the backend and stable algorithm ID dispatch to
   `crates/compare-demosaic/cpp/shim.cpp` and its header. Add a
   `ReferenceBackend` value only when the comparison uses a new reference
   library.
5. Add the Fornax runner, `Case`, and `CASES` entry in
   `crates/compare-demosaic/src/cases.rs`. Keep the algorithm/reference ID pair
   unique.
6. Run the new case:

   ```bash
   cargo run --release -p compare-demosaic -- run --algorithm <id>
   ```

7. Run the crate tests and update only intended snapshots after review:

   ```bash
   cargo insta test -p compare-demosaic
   cargo insta accept
   ```

8. Update the algorithm's hand-written README note when its assumptions,
   reference, matches, or known differences need explanation. Do not edit text
   inside generated markers.
9. Refresh the full baseline and generated README:

   ```bash
   cargo run --release -p compare-demosaic -- run
   cp target/compare-demosaic/canonical.json \
     crates/compare-demosaic/baseline/canonical.json
   cargo run --release -p compare-demosaic -- readme
   ```

10. Verify the generated output and comparison:

    ```bash
    cargo run --release -p compare-demosaic -- readme --check
    cargo run --release -p compare-demosaic -- diff \
      crates/compare-demosaic/baseline/canonical.json \
      target/compare-demosaic/canonical.json
    ```

11. Review `git diff`, commit the adapter, Rust case, snapshots, baseline,
    generated README regions, and complete note in the same pull request.
