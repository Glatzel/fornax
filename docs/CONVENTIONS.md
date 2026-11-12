# Documentation Conventions

This guide defines where Fornax documentation belongs and how new files should be named and structured.

## Structure

Documentation is organized by purpose:

| Folder | Use for |
| --- | --- |
| `architecture/` | Current system boundaries, interfaces, and data flow. |
| `decoder/` | Decoder operations, corpus work, benchmarks, and format support plans. |
| `reference/` | Durable pinned-source and implementation-behavior research. |
| `plans/` | Active per-work implementation plans. |
| `_archive/` | Completed plans, historical records, and design provenance. |
| `decisions/` | Architecture decision records, if the project starts using them. |

The maximum folder depth below `docs/` is two. Prefer an existing folder over
creating a new one, and do not add a new top-level topic for a single document
unless it defines a durable documentation category.

## Naming

- Use lowercase ASCII kebab-case for files and folders.
- Use descriptive names such as `dng-compression.md`, not `new`, `final`,
  `misc`, or `temp`.
- Do not put dates or version numbers in ordinary filenames.
- Name ADRs `0001-title.md` under `decisions/`.
- `README.md`, `CONVENTIONS.md`, and `_archive/` are intentional exceptions.
- Keep names aligned with the document's H1 and purpose.

## Document shape

Every document starts with exactly one H1 followed by one sentence that
summarizes its purpose. Additional detail belongs under level-two sections.

Use relative Markdown links for repository documentation. Verify targets and
fragments after moving or renaming a document.

## Where new documents belong

| New document | Location |
| --- | --- |
| Crate boundaries, interfaces, or pipeline flow | `architecture/` |
| Decoder-independent benchmark or corpus process | `decoder/` |
| Format-specific implementation scope | `decoder/formats/` |
| Demosaic or other algorithm research | `reference/algorithms/` |
| Decoder-specific source research | `reference/decoders/` |
| Codec or compression research | `reference/codecs/` |
| Active implementation plan | `plans/` |
| Architecture decision record | `decisions/` |
| Completed plan or superseded design | `_archive/` |

## Archive policy

Archive completed plans and superseded evidence instead of deleting them.
Archived material preserves implementation history but is not authoritative for
new work. Link archived documents from `docs/README.md` and describe them as
historical.

When unsure whether a document remains relevant, move it to the closest
`_archive/` category and flag it for maintainer review.
