# Rust Demosaicing Crates and Interfaces

> **Task:** `RES-08`
> **Owner:** primary agent
> **Status:** `paused` (split evidence preserved; acceptance review deferred)
> **Last updated:** 2026-10-23
> **Upstream versions:** `naorunaoru/demosaic`
> `5050131e998b7f6414eef02eeb60f74b99d77451`; `kaimai-apex/meraraw-demosaic`
> `b94ebd4324ce56218e42989a5ed852b32a52003f`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Bayer input/output and border contracts are source-verified.
> - [x] Validation, errors, and allocation behavior are documented.
> - [x] Reusable code and ideas are distinguished with licenses.
> - [x] Maintenance and maturity are supported by evidence.
> - [x] Non-target modes are excluded from the initial recommendation.
> - [x] The report contains the six required sections and cites sources.

## 1. Scope and questions

This report evaluates Rust demosaicing crates and interface patterns for
conventional 2x2 RGB Bayer data. Algorithm quality and the initial algorithm set
remain owned by `RES-05`.

The questions were:

- What Bayer input/output and border contracts are exposed?
- How are buffer sizes, CFA support, and dimensions validated?
- Which API patterns preserve independent algorithm implementations?
- What test patterns are reusable?
- What does each implementation add beyond the existing `fornax-dalim`?

## 2. Findings

### 2.1 `demosaic` crate

`[verified]` The crate exposes a buffer-oriented `f32` API: single-channel
row-major CFA input, dimensions, `CfaPattern`, an `Algorithm` enum, and
caller-provided planar output. It validates input/output lengths and returns
typed errors for size mismatch, unsupported combinations, and small X-Trans
images (`src/lib.rs:137-270`).

`[verified]` It supports Bayer bilinear, MHC, PPG, AHD, and VNG; Quad Bayer
bilinear/Quad-PPG; and X-Trans bilinear/Markesteijn/DHT. CFA patterns include a
shift operation for crop offsets. It does not parse RAW files, preprocess black
levels, or retain metadata.

`[verified]` Its MHC implementation falls back to bilinear within two pixels of
the border and preserves the known sample elsewhere
(`src/bayer/mhc_impl.rs:12-60,70-145`).

`[inference]` The algorithm/buffer separation is a useful API precedent, but a
single closed `Algorithm` enum couples registry and implementation. `fornax`
should keep independently implementable strategies.

### 2.2 MeraRAW Demosaic

`[verified]` MeraRAW separates a core RAW/image model, demosaic crate, I/O, and
UI crates. Its demosaic trait exposes ID/name/description, supported CFA, cost,
minimum size, implementation status, and `run` returning a typed result.
`check` validates pattern support, minimum dimensions, and buffer length
(`crates/miro-demosaic/src/lib.rs:48-145`).

`[verified]` `DemosaicOptions` keeps shared post-processing controls and
algorithm-specific tuning outside individual implementations, and the registry
supports stubs and runtime capability inspection
(`crates/miro-demosaic/src/lib.rs:145-275`).

`[verified]` Its application pipeline is explicitly
`crop -> white balance -> demosaic -> camera-to-XYZ -> ... -> display`. That is
broader than `fornax`: white balance, color conversion, tone mapping, and display
orientation remain outside scope.

`[inference]` MeraRAW is useful for trait shape, validation, registry, and
synthetic-quality-test ideas. Its application scope and young API should not be
imported wholesale.

### 2.3 Reuse and testing

`[verified]` Both projects return errors for buffer size, CFA support, and
dimension preconditions instead of relying only on panics. MeraRAW adds
synthetic mosaicing, PSNR, SSIM, zipper metrics, and tiled/untiled comparison
(`crates/miro-demosaic/src/testing.rs`; `tests/correctness.rs`).

`[inference]` Reusable test pattern: generate known RGB ground truth, mosaic it
with each supported Bayer phase, demosaic, compare deterministic metrics and
selected exact pixels, test invalid buffers and layouts, compare tiled and
untiled paths, and keep visual checks supplementary.

| Project | Reusable code | Reusable ideas | License |
| --- | --- | --- | --- |
| `demosaic` | Potentially for comparison or compatible algorithms | Buffer validation, planar output, algorithm registration lessons | MIT OR Apache-2.0 |
| MeraRAW | Possible components after audit | Stateless trait, registry/capability reporting, synthetic tests | Apache-2.0 |

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| `demosaic` buffer API and validation | `demosaic/src/lib.rs:137-270` |
| `demosaic` MHC border behavior | `demosaic/src/bayer/mhc_impl.rs:12-60,70-145` |
| MeraRAW trait, options, registry | `meraraw-demosaic/crates/miro-demosaic/src/lib.rs:48-275` |
| MeraRAW broader pipeline | `meraraw-demosaic/README.md`, "Architecture" |
| Synthetic quality approach | `crates/miro-demosaic/src/testing.rs`; `tests/correctness.rs` |

## 4. Implications for fornax

`[recommendation]` Keep the shared interface open to independent Bayer algorithm
implementations and return structured errors for unsupported layout, dimensions,
or buffers.

`[recommendation]` Keep options and allocation policy explicit; do not make a
closed algorithm enum part of the core API solely to mirror a reference crate.

`[recommendation]` Use synthetic mosaics and tiled/untiled comparisons as
correctness evidence. Algorithm choice and quality thresholds remain `RES-05`
and design decisions.

## 5. Open questions and limitations

- Comparative image quality was not independently reproduced.
- MeraRAW's implementation-status claims were not exhaustively verified against
  every algorithm path.
- X-Trans, Quad Bayer, and other non-target modes are noted only to avoid
  accidental inclusion in the initial Bayer recommendation.
- Final object safety, allocation, options, and metadata-return decisions belong
  to design after synthesis.

## 6. References

- `demosaic`: `https://github.com/naorunaoru/demosaic.git` at
  `5050131e998b7f6414eef02eeb60f74b99d77451`; MIT OR Apache-2.0.
- MeraRAW Demosaic: `https://github.com/kaimai-apex/meraraw-demosaic.git` at
  `b94ebd4324ce56218e42989a5ed852b32a52003f`; Apache-2.0.
- Pinned checkouts: `agent-work/references/other/demosaic-crate/` and
  `agent-work/references/other/meraraw-demosaic/`.
