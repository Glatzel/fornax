# RAW Pipeline Design Index

Each `DES-*` task writes a proposal named `<TASK-ID>-<slug>.md` in this
directory. This split keeps each design branch reviewable and avoids concurrent
edits to one shared document. The authoritative task contracts are in
`agent-work/tasks/raw-pipeline/DES-*.toml`.

| Task | Deliverable |
| --- | --- |
| `DES-001` | `DES-001-universal-raw-representation.md` |
| `DES-002` | `DES-002-metadata-provenance.md` |
| `DES-003` | `DES-003-cfa-indexing.md` |
| `DES-004` | `DES-004-preprocessing-state.md` |
| `DES-005` | `DES-005-demosaicing-interface.md` |
| `DES-006` | `DES-006-crate-architecture.md` |
| `DES-007` | `DES-007-design-review.md` |

`DES-007` also owns the consolidated
[`../raw-pipeline.md`](../raw-pipeline.md), which it assembles only after the
component proposals have merged. Design decisions must cite merged `RES-*` or
`SYN-*` pull requests and must not resolve unresolved metadata, coordinate,
sample-domain, or preprocessing questions by assumption.

The decoder architecture must expose existing implementations as selectable
backends behind a `fornax`-owned contract. LibRaw and dnglab/rawler are decoder
candidates, `dnc` is a conversion bridge, and darktable/rawspeed is eligible
only through an isolated decoder boundary. Do not design a first-party RAW
format decoder for the initial pipeline.

Do not begin implementation before the user approves and merges `DES-007`.
