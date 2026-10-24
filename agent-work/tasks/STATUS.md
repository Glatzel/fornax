# RAW Pipeline Task Status

**Snapshot:** 2026-10-24

This is the manually maintained task-status overview for the RAW pipeline. It
is updated when a task PR opens and when dependency PRs merge.

`Done` means the task deliverable is complete and its task PR exists; the PR may
still be open. GitHub remains the review and merge authority. A downstream task
becomes `Ready` only after every dependency task is `Done` and every dependency
PR is merged.

## Status legend

| Status | Meaning |
| --- | --- |
| `Blocked` | A dependency is not `Done`, or a dependency task PR is still open. |
| `Ready` | Every dependency is `Done` and merged; work has not started. |
| `Done` | The task deliverable is complete and its task PR exists, whether open or merged. |

## Summary

| Phase | Tasks | Blocked | Ready | Done |
| --- | ---: | ---: | ---: | ---: |
| Research | 9 | 0 | 0 | 9 |
| Synthesis | 3 | 0 | 0 | 3 |
| Design | 6 | 3 | 0 | 3 |
| Implementation | 12 | 12 | 0 | 0 |
| QA | 6 | 6 | 0 | 0 |
| **Total** | **36** | **21** | **0** | **15** |

## Research

| ID | Task | Depends on | Status | Artifact / PR |
| --- | --- | --- | --- | --- |
| `RES-01` | [Inventory existing components](raw-pipeline/RES-01.toml) | none | `Done` | [report](../research/RES-01-fornax-inventory.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-02` | [Research LibRaw](raw-pipeline/RES-02.toml) | none | `Done` | [report](../research/RES-02-libraw.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-03` | [Research dnglab and DNG metadata](raw-pipeline/RES-03.toml) | none | `Done` | [report](../research/RES-03-dnglab.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-04` | [Research darktable preprocessing](raw-pipeline/RES-04.toml) | none | `Done` | [report](../research/RES-04-darktable.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-05` | [Research demosaicing](raw-pipeline/RES-05.toml) | none | `Done` | [report](../research/RES-05-demosaicing.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-06` | [Survey Rust RAW decoders](raw-pipeline/RES-06.toml) | none | `Done` | [report](../research/RES-06-rust-raw-decoders.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-07` | [Survey Rust TIFF and DNG components](raw-pipeline/RES-07.toml) | none | `Done` | [report](../research/RES-07-rust-dng-tiff.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-08` | [Survey Rust demosaicing crates](raw-pipeline/RES-08.toml) | none | `Done` | [report](../research/RES-08-rust-demosaicing.md), [#534](https://github.com/Glatzel/fornax/pull/534) |
| `RES-09` | [Recommend selectable raw decoder backends](raw-pipeline/RES-09.toml) | `RES-01`–`RES-04`, `RES-06`–`RES-08` | `Done` | [report](../research/RES-09-rust-ecosystem.md), [#537](https://github.com/Glatzel/fornax/pull/537) |

## Synthesis

| ID | Task | Depends on | Status | Artifact / PR |
| --- | --- | --- | --- | --- |
| `SYN-01` | [Reconcile metadata and sample semantics](raw-pipeline/SYN-01.toml) | `RES-01`–`RES-04`, `RES-06`, `RES-07` | `Done` | [note](../research/notes/SYN-01.md), [#540](https://github.com/Glatzel/fornax/pull/540) |
| `SYN-02` | [Reconcile preprocessing and black levels](raw-pipeline/SYN-02.toml) | `RES-02`, `RES-04`, `SYN-01` | `Done` | [note](../research/notes/SYN-02.md), [#541](https://github.com/Glatzel/fornax/pull/541) |
| `SYN-03` | [Synthesize cross-source recommendations](raw-pipeline/SYN-03.toml) | all `RES-*`, `SYN-01`, `SYN-02` | `Done` | [summary](../research/SYN-03-summary.md), [#546](https://github.com/Glatzel/fornax/pull/546) |

## Design

| ID | Task | Depends on | Status | Proposal / PR |
| --- | --- | --- | --- | --- |
| `DES-001` | Design universal RAW representation | `SYN-01`, `SYN-02`, `DES-003` | `Done` | [proposal](../design/raw-pipeline/DES-001-universal-raw-representation.md), [#545](https://github.com/Glatzel/fornax/pull/545) |
| `DES-002` | Design metadata and provenance | `SYN-01` | `Done` | [proposal](../design/raw-pipeline/DES-002-metadata-provenance.md), [#542](https://github.com/Glatzel/fornax/pull/542) |
| `DES-003` | Design Bayer CFA model and indexing | `SYN-01` | `Done` | [proposal](../design/raw-pipeline/DES-003-cfa-indexing.md), [#543](https://github.com/Glatzel/fornax/pull/543) |
| `DES-004` | Design preprocessing and processing state | `SYN-01`, `SYN-02`, `DES-001`, `DES-003` | `Blocked` | [manifest](raw-pipeline/DES-004.toml) |
| `DES-005` | Design the demosaicing interface | `SYN-01`, `SYN-03`, `DES-001`, `DES-003` | `Blocked` | [manifest](raw-pipeline/DES-005.toml) |
| `DES-006` | Design crate boundaries | `SYN-03`, `DES-001`, `DES-004`, `DES-005` | `Blocked` | [manifest](raw-pipeline/DES-006.toml) |

## Implementation

| ID | Task | Depends on | Status | Manifest |
| --- | --- | --- | --- | --- |
| `IMPL-DEC-001` | Define the decoder adapter interface | `DES-001`, `DES-006` | `Blocked` | [manifest](raw-pipeline/IMPL-DEC-001.toml) |
| `IMPL-DEC-002` | Adapt the LibRaw backend | `IMPL-DEC-001` | `Blocked` | [manifest](raw-pipeline/IMPL-DEC-002.toml) |
| `IMPL-DEC-003` | Map source metadata to universal metadata | `IMPL-DEC-001`, `DES-002`, `DES-003` | `Blocked` | [manifest](raw-pipeline/IMPL-DEC-003.toml) |
| `IMPL-DEC-004` | Add decoder validation and regressions | `IMPL-DEC-002`, `IMPL-DEC-003` | `Blocked` | [manifest](raw-pipeline/IMPL-DEC-004.toml) |
| `IMPL-PRE-001` | Implement scalar black-level correction | `DES-001`, `DES-004` | `Blocked` | [manifest](raw-pipeline/IMPL-PRE-001.toml) |
| `IMPL-PRE-002` | Implement CFA-aware black-level correction | `IMPL-PRE-001`, `DES-003` | `Blocked` | [manifest](raw-pipeline/IMPL-PRE-002.toml) |
| `IMPL-PRE-003` | Implement spatially varying black levels | `IMPL-PRE-002` | `Blocked` | [manifest](raw-pipeline/IMPL-PRE-003.toml) |
| `IMPL-PRE-004` | Implement additional approved preprocessing | `DES-004` | `Blocked` | [manifest](raw-pipeline/IMPL-PRE-004.toml) |
| `IMPL-DEM-001` | Implement bilinear demosaicing | `DES-005` | `Blocked` | [manifest](raw-pipeline/IMPL-DEM-001.toml) |
| `IMPL-DEM-002` | Implement Malvar–He–Cutler demosaicing | `DES-005` | `Blocked` | [manifest](raw-pipeline/IMPL-DEM-002.toml) |
| `IMPL-DEM-003` | Implement selected edge-aware demosaicing | `DES-005`, `RES-05` | `Blocked` | [manifest](raw-pipeline/IMPL-DEM-003.toml) |
| `IMPL-INT-001` | Integrate the end-to-end pipeline | `IMPL-DEC-004`, `IMPL-PRE-002`, one `IMPL-DEM-*` | `Blocked` | [manifest](raw-pipeline/IMPL-INT-001.toml) |

## QA

| ID | Task | Depends on | Status | Manifest |
| --- | --- | --- | --- | --- |
| `QA-001` | Validate metadata preservation | `IMPL-INT-001` | `Blocked` | [manifest](raw-pipeline/QA-001.toml) |
| `QA-002` | Add numerical regression coverage | `IMPL-INT-001` | `Blocked` | [manifest](raw-pipeline/QA-002.toml) |
| `QA-003` | Validate error paths and unsupported inputs | `IMPL-INT-001` | `Blocked` | [manifest](raw-pipeline/QA-003.toml) |
| `QA-004` | Establish performance baselines | `IMPL-INT-001` | `Blocked` | [manifest](raw-pipeline/QA-004.toml) |
| `QA-005` | Document behavior and API migration | `IMPL-INT-001` | `Blocked` | [manifest](raw-pipeline/QA-005.toml) |
| `QA-006` | Run final workspace validation | `IMPL-DEC-004`, `IMPL-INT-001`, `QA-001`–`QA-005` | `Blocked` | [manifest](raw-pipeline/QA-006.toml) |

## Update rules

1. When a task branch is pushed and its PR is opened, mark the task `Done` and
   add the report/proposal link plus PR number.
2. Keep dependent tasks `Blocked` while any dependency PR remains open.
3. After every dependency PR is merged, mark the dependent task `Ready` only
   when no worktree or task PR has been created for it.
4. When a `Ready` task opens its PR, mark it `Done` and repeat the process for
   its dependents.
5. Do not add an intermediate worktree/PR status. The presence of a worktree or
   open PR can be inferred from the branch, PR link, and `Done` state.

```mermaid
stateDiagram-v2
    state "Blocked" as Blocked
    state "Ready" as Ready
    state "Done" as Done

    [*] --> Blocked: dependency incomplete or unmerged
    Blocked --> Ready: every dependency is Done and merged
    Ready --> Done: task PR opened
    Done --> [*]
```
