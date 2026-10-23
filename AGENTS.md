# Agent Instructions — fornax

## Project identity

`fornax` is a Rust-first RAW image processing workspace (edition 2024, resolver 3). The intended pipeline is:

1. **Decode** a supported RAW file into a universal, backend-independent RAW representation with enough metadata for later stages.
2. **Preprocess** the sensor-domain samples using explicit metadata semantics (notably black-level correction).
3. **Demosaic** the sensor mosaic into a reconstructed image.

The intended scope stops at demosaicing. Do not silently add white-balance application, color correction, color-space conversion, tone mapping, denoising, sharpening, or display rendering.

Current crates, as verified at initialization:

- `fornax-core` — foundational traits and types: `IDecoder<T>`, `IPostProcessor`, `BayerImage<T>`, `BayerPattern` (RGGB, BGGR, GRBG, GBRG), `FornaxPrimitive`, `FornaxError`.
- `fornax-dalim` — Rust-native post-processing: `IDemosaic<T>`, `DemosaicLinear`, `Dalim`, and the `demosaic` benchmark.
- `libraw` / `libraw-sys` — safe LibRaw wrapper and generated bindings. `Rawdata` currently exposes mono/RGB/RGBA buffers; typed metadata access is minimal.
- `fornax` — high-level manager combining a decoder and a post-processor.
- `fornax-py` — PyO3 Python bindings.
- `dnc` — Adobe DNG Converter wrapper.
- `fornax-devtool` — development utilities.

The repository does not yet have a universal RAW representation or metadata-driven preprocessing. Do not assume the current `BayerImage` is the final shared model.

## Development principles

- Inspect and reuse existing crates before proposing new ones.
- Keep decoder-specific representations behind adapters.
- Keep shared RAW types independent of any specific decoder backend, including LibRaw.
- Use explicit, statically typed APIs and well-defined numerical semantics.
- Preserve useful metadata even when the current pipeline does not consume it.
- Prefer safe Rust and document necessary unsafe code and its invariants.
- Avoid unnecessary dependencies, abstractions, allocations, and buffer copies.
- Preserve existing public APIs when practical; document every intentional breaking change.
- Add tests for numerical correctness, edge cases, and regressions.
- Use the repository's actual formatting, lint, build, and test commands.

## Build, test, and lint commands

These commands reflect the checked-in CI configuration and tooling:

- **Setup**: `git submodule update --init --recursive`, then `pixi install`. `scripts/setup.ps1` performs this and sets LibRaw environment variables.
- **Rust tests**: `cargo test --workspace --all-features --exclude fornax-py`
- **Clippy**: `cargo clippy --workspace --all-features --exclude fornax-py`
- **Docs**: `RUSTFLAGS="-Dwarnings" cargo doc --workspace --all-features --no-deps`
- **Format**: `cargo +nightly fmt` and `cargo +nightly fmt --check` (`rustfmt.toml` enables unstable options, so nightly rustfmt is required).
- **Benchmarks**: `cargo bench -p fornax-dalim --bench demosaic`
- **Python**: `pwsh ./scripts/maturin-develop.ps1`, then `pytest` from `crates/fornax-py`.

Record actual results. If a command cannot be run, say so explicitly instead of implying success.

## Agent workflow

- Agent materials live in `agent-work/`. The hidden `.agents/` directory is mounted read-only in the agent runtime, so do not attempt to write there.
- Read `agent-work/raw-pipeline.md` before beginning work on the RAW pipeline. It is the authoritative task specification and the single source of truth; do not create competing task specifications.
- Check `agent-work/tasks/raw-pipeline.md` for live task status, dependencies, and evidence. Only `RES-01` through `RES-06` are initially ready; synthesis, design, and implementation tasks are blocked on upstream work.
- Each reference has exactly one report under `agent-work/research/`. See `agent-work/research/README.md` for report ownership, prerequisites, and acceptance criteria.
- Research owners write only their assigned report. `SYN-01` and `SYN-02` write only their own files under `agent-work/research/notes/`; `SYN-03` owns `agent-work/research/summary.md`. Never edit another task's report.
- Do not start synthesis before its prerequisite reports are accepted, do not finalize design before `SYN-03`, and do not start implementation before `DES-007` is accepted.
- Do not implement shared APIs before the research and design have been sufficiently reviewed.
- Keep the implementation roadmap synchronized with actual progress; never mark work complete without evidence.
- Define stable interfaces before assigning independent implementation tasks to parallel workers.
- Avoid overlapping edits to shared types, manifests, and public interfaces.
- Report evidence, test results, blockers, and unresolved decisions explicitly.
- Do not fabricate research or design reports. Placeholder files describe required structure only.

## Scope discipline

- Keep decoding, metadata interpretation, preprocessing, and demosaicing separate.
- A caller must not be able to apply the same preprocessing step twice without detection or an explicit, documented opt-in.
- Do not apply white balance or camera color matrices merely because the metadata is present.
- Support only CFA layouts and sample encodings that are explicitly designed for; reject the rest with structured errors.
- Preserve metadata across stage transitions.

The full requirements — research questions, design deliverables, task breakdown, and validation protocol — live in `agent-work/raw-pipeline.md`. Do not duplicate them here.
