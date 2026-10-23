# dnglab and DNG Metadata

> **Task:** `RES-03`
> **Owner:** primary agent
> **Status:** `in_progress` — resumed after primary-source network access
> became available.
> **Last updated:** 2026-10-23
> **Upstream versions:** no dnglab version or commit could be inspected. The
> available DNG-adjacent evidence is LibRaw `0.22.2`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Relevant DNG tags are mapped to dnglab source symbols with units,
>   defaults, and coordinate conventions.
> - [ ] Active area, black-level patterns, white levels, and orientation are
>   covered from dnglab behavior or explicitly excluded with a reason.
> - [ ] DNG versus vendor RAW metadata differences are identified.
> - [ ] Licensing constraints for dnglab are recorded.
> - [x] All six required report sections are complete.
> - [x] Missing evidence is recorded instead of invented.

## 1. Scope and questions

Investigate dnglab and DNG metadata interpretation. The questions were:

- How does dnglab parse DNG files and expose the raw image?
- What do `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `CFAPattern`,
  `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`, and `Orientation` mean,
  including units, defaults, and coordinate conventions?
- How do active-area and cropped-image coordinates relate to the stored buffer?
- How are black-level patterns aligned with CFA phase?
- Which tags does dnglab normalize, preserve, or discard?
- Where does the DNG specification leave behavior ambiguous?
- What licensing constraints apply to any considered reuse?

## 2. Findings

### 2.1 Offline source availability

`[verified]` No dnglab source, crate cache, or vendored copy is present in this
environment:

- `find /home/glatzel -maxdepth 5 -iname '*dnglab*'` returned only
  `agent-work/research/dnglab.md`.
- `grep -i 'dnglab|rawler|rawloader' Cargo.lock` returned no matches.
- `~/.cargo` does not exist; there is no local Rust crate registry to inspect.
- `curl https://raw.githubusercontent.com/LibRaw/LibRaw/master/README.md`
  failed with `Could not connect to server` through proxy `127.0.0.1:...`.
- The runtime's `fetch_url` and `web_search` tools are unavailable, and
  `agent_open` is unsupported, so no delegated online research was possible.

`[verified]` Consequently, dnglab's parser, `RawImage` representation, CFA
handling, active-area logic, orientation handling, and license metadata could
not be inspected. No dnglab-specific claim is made in this report.

### 2.2 DNG-adjacent evidence available from LibRaw

`[verified]` LibRaw 0.22.2 exposes a DNG version field:
`libraw_iparams_t.dng_version` is an `unsigned` at
`libraw_types.h:198`. The local wrapper exposes it as `IParams::dng_version()`
(`crates/libraw/src/data_structure/iparams.rs:84`).

`[verified]` LibRaw 0.22.2 exposes a DNG-specific levels structure,
`libraw_dng_levels_t`, at `libraw_types.h:254-270`:

- `unsigned dng_cblack[LIBRAW_CBLACK_SIZE]` (line 257).
- `unsigned dng_black` (line 258).
- `float dng_fcblack[LIBRAW_CBLACK_SIZE]` (line 259).
- `float dng_fblack` (line 260).
- `unsigned dng_whitelevel[4]` (line 261).
- `ushort default_crop[4]` with the comment "Origin and size" (line 262).
- `float user_crop[4]` with the comment "top-left-bottom-right relative to
  default_crop" (line 263).
- `unsigned preview_colorspace` (line 264).
- `float analogbalance[4]` (line 265), `float asshotneutral[4]` (line 266),
  `float baseline_exposure` (line 267), `float LinearResponseLimit` (line 268).

`[verified]` `LIBRAW_CBLACK_SIZE` is `4104` (`libraw_const.h:85`), and
`libraw_colordata_t` embeds `libraw_dng_levels_t dng_levels` at
`libraw_types.h:826`.

`[inference]` The field names strongly suggest a mapping to DNG concepts:
`dng_black`/`dng_cblack` to `BlackLevel`, `dng_whitelevel` to `WhiteLevel`,
`default_crop`/`user_crop` to `DefaultCropOrigin`/`DefaultCropSize`, and
`asshotneutral` to `AsShotNeutral`. This is name-based inference only. The
installed headers do not state units, default values, repeat-pattern layout,
coordinate origin, or how LibRaw normalizes the tags.

`[verified]` The local wrapper does not expose `dng_levels`, `dng_cblack`,
`dng_whitelevel`, or `default_crop`; it exposes only `dng_version`
(`crates/libraw/src/data_structure/iparams.rs:84`). Therefore the current
`fornax` code cannot read DNG black/white/crop metadata through its wrapper.

### 2.3 What cannot be concluded

`[inference]` LibRaw's DNG fields are not a substitute for dnglab's behavior.
LibRaw may normalize, rescale, or reinterpret DNG tags before exposing these
structures. Without dnglab source and the DNG specification, neither the
normalization nor the equivalent behavior can be established.

`[verified]` No evidence was found for dnglab's `RawImage` sample type, CFA
representation, active-area computation, orientation handling, or metadata
preservation.

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| No dnglab source or cache | `find /home/glatzel -maxdepth 5 -iname '*dnglab*'`; `Cargo.lock` search; absent `~/.cargo` |
| Network unavailable | `curl` failure to `raw.githubusercontent.com`; `fetch_url`/`web_search` unavailable |
| LibRaw DNG version field | `libraw_types.h:198`; `iparams.rs:84` |
| LibRaw DNG levels structure | `libraw_types.h:254-270` |
| `LIBRAW_CBLACK_SIZE` | `libraw_const.h:85` |
| DNG levels embedded in color data | `libraw_types.h:826` |
| Wrapper exposes only `dng_version` | `crates/libraw/src/data_structure/iparams.rs:84` |
| No dnglab behavioral claims | No source, test, or specification available |

## 4. Implications for fornax

`[recommendation]` Do not finalize a DNG adapter, DNG metadata mapping, or
DNG-specific coordinate model from this report. The current evidence is
insufficient.

`[recommendation]` Keep the following provisional checklist for the eventual
dnglab and DNG investigation:

- `BlackLevel`: scalar, per-channel, repeat dimensions, units, and sample
  domain.
- `WhiteLevel`: per-channel values, bit depth, and relationship to saturation.
- `CFAPattern` / `CFARepeatPatternDim`: layout, phase, and origin.
- `ActiveArea`: coordinate basis, inclusive/exclusive bounds, and relation to
  the stored buffer.
- `DefaultCropOrigin` / `DefaultCropSize`: units, rounding, and relation to
  `ActiveArea`.
- `Orientation`: rotation/mirror semantics and where they are applied.
- DNG version handling and backward compatibility.
- Which metadata dnglab preserves, normalizes, or discards.

`[recommendation]` Treat LibRaw's `dng_levels` as one source of DNG-related
evidence, not as the specification. Any universal DNG semantics must be
reconciled by `SYN-01` and `SYN-02` after `RES-03` is completed.

`[recommendation]` Do not add dnglab as a dependency or copy code from it until
its license and API behavior are inspected.

## 5. Open questions and limitations

- Which dnglab version, commit, and license should be inspected?
- How does dnglab represent `RawImage` samples and their valid range?
- Does dnglab expose DNG tags directly, normalize them, or discard them?
- What are the exact units, defaults, and coordinate conventions for
  `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `ActiveArea`, and
  `DefaultCrop*`?
- How are black-level patterns aligned with CFA phase and cropped coordinates?
- How does dnglab handle orientation, and at which stage is it applied?
- How do DNG metadata semantics differ from vendor RAW metadata in LibRaw?
- Which DNG tags does dnglab preserve for a universal representation?

`[verified]` This report cannot answer any dnglab-specific question above. It
records the environment limitation and the available LibRaw declarations so the
next attempt does not start from zero.

## 6. References

- DNG-adjacent evidence:
  - `~/.pixi/envs/base/include/libraw/libraw_types.h:198, 254-270, 826`
  - `~/.pixi/envs/base/include/libraw/libraw_const.h:85`
  - `crates/libraw/src/data_structure/iparams.rs:84`
- Local environment checks:
  - `find /home/glatzel -maxdepth 5 -iname '*dnglab*'`
  - `grep -i 'dnglab|rawler|rawloader' Cargo.lock`
  - `ls ~/.cargo`
  - `curl` probe against `raw.githubusercontent.com`
- dnglab repository URL, version, commit, tests, and license: not available
  offline; must be recorded when access is restored.
- DNG specification revision and relevant tag sections: not available offline;
  must be recorded when access is restored.
