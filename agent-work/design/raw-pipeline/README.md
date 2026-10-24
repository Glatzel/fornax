# RAW Pipeline Design History

Each `DES-*` task writes a proposal named `<TASK-ID>-<slug>.md` in this
directory. These proposals retain design rationale and review history. The
normative, persistent design lives in
[`docs/raw-pipeline/`](../../../docs/raw-pipeline/README.md).

| Task | Deliverable |
| --- | --- |
| `DES-001` | `DES-001-universal-raw-representation.md` |
| `DES-002` | `DES-002-metadata-provenance.md` |
| `DES-003` | `DES-003-cfa-indexing.md` |
| `DES-004` | `DES-004-preprocessing-state.md` |
| `DES-005` | `DES-005-demosaicing-interface.md` |
| `DES-006` | `DES-006-crate-architecture.md` |

Design decisions must cite merged `RES-*` or `SYN-*` pull requests and must not
resolve unresolved metadata, coordinate, sample-domain, or preprocessing
questions by assumption.

Every design change must update the corresponding persistent
`docs/raw-pipeline/` guide in the same PR. If a proposal conflicts with those
guides, the persistent guide is authoritative and the PR must reconcile them.

The decoder architecture must expose existing implementations as selectable
backends behind a `fornax`-owned contract. LibRaw and dnglab/rawler are decoder
candidates, `dnc` is a conversion bridge, and darktable/rawspeed is eligible
only through an isolated decoder boundary. Do not design a first-party RAW
format decoder for the initial pipeline.

Implementation begins only after the relevant `DES-*` PRs and the persistent
design-docs PR are merged.
