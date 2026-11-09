# Documentation impact

## Inventory method

`[CURRENT]` The baseline contains 89 Markdown files outside `external/`,
`target/`, `.git/`, dependency/workspace-copy folders, and this new
`docs/proposal/` directory.

`[CURRENT]` The inventory command is
`find . \( -path './external' -o -path './target' -o -path './.git' -o -path './.worktrees' -o -path './.pixi' -o -path './docs/proposal' \) -prune -o -type f -name '*.md' -print | sort`.

`[CURRENT]` Signal columns use `rg -il` against these families: crate names
(`K`), workspace/layout (`L`), preprocess (`P`), demosaic/postprocess (`D`),
DNG (`N`), and facade/re-export paths (`F`). `-` means no match. The exact
patterns are reproduced with:

```text
K: rg -il 'fornax-(core|codec|tiff|devtool|dng|arw|decoder|preprocessor|postprocessor|cli|py)|`fornax`|compare-demosaic|decoder-tracker' FILE
L: rg -il 'workspace|Cargo.toml|members|worktree|crate map|layout' FILE
P: rg -il 'preprocessor|preprocess' FILE
D: rg -il 'postprocessor|postprocess|demosaic' FILE
N: rg -il '\bDNG\b|fornax-dng|DngTag|DngWriter' FILE
F: rg -il 'facade|fornax::|crates/fornax/' FILE
```

`[CURRENT]` The purpose column is condensed from each file's first Markdown
heading, reproduced with `rg -m1 '^#{1,2} ' FILE`.

`[TARGET]` Every action in the table is proposed post-refactor work. `None`
means retain the document as written, usually because it is a historical plan
or research note.

## Impact table

| File | Purpose | Signals | `[TARGET]` action | Size | Stale now |
| --- | --- | --- | --- | --- | --- |
| `.agents/skills/decoder-corpus-tracker/SKILL.md` | Corpus tracker procedure | K,N | Update workspace-root/package commands if paths change | S | No |
| `.agents/skills/demosaic-comparison/SKILL.md` | Comparison workflow | K,D | Update imports/paths and generated-table commands | S | No |
| `AGENTS.md` | Agent/repository invariants | K,L,P,D,N,F | Rewrite ownership, workspace, facade, and validation sections | L | Yes: `AGENTS.md:25` non-generic `Decoder`; `AGENTS.md:28` non-member decoder crates, while membership is `Cargo.toml:3` |
| `CHANGELOG.md` | Release history | N,F | Add future refactor entry only at release time; do not rewrite history | S | No |
| `README.md` | Project overview | K,L,P,D,N,F | Rewrite component map, Python status, facade/package name, and workspace split | L | Yes: `README.md:23` misplaces DNG in core; implementation is `crates/fornax-dng/src/lib.rs:7` |
| `TODO.md` | Human backlog | K,P,D,N | Repoint future work to target crate names; preserve task IDs | M | No |
| `book/AGENTS.md` | Book editing rules | L | Update source-link workflow only if crate paths move | S | No |
| `book/README.md` | Human-facing pipeline book | K,P,D | Update stage-to-crate map while preserving conceptual stages | M | No |
| `book/figures/README.md` | Figure generation | L | No crate API change expected | None | No |
| `book/src/SUMMARY.md` | Book chapter list | P,D | Repoint chapter links only if book location changes | None | No |
| `book/src/bibliography.md` | External references | K,D,N | Update compare crate link if tools move | S | No |
| `book/src/glossary.md` | Pipeline glossary | K,L,P,D | Repoint core/Bayer, preprocessor, and postprocessor links | M | No |
| `book/src/notation.md` | Notation and source links | K,L,P,D | Repoint core, preprocessor, and postprocessor links | M | No |
| `book/template.md` | Chapter template | K,D | Update compare link if package path changes | S | No |
| `crates/compare-demosaic/AGENTS.md` | Comparison maintainer guide | K,L,P,D,N,F | Update dependencies, commands, and generated README workflow | M | No |
| `crates/compare-demosaic/README.md` | Comparison tool/results | K,L,P,D,N,F | Regenerate generated sections and refresh API descriptions | M | No |
| `crates/decoder-tracker/AGENTS.md` | Tracker maintainer guide | K,L,N,F | Update Cargo/package paths after workspace split | S | No |
| `crates/decoder-tracker/README.md` | Tracker usage/results | K,L,N,F | Regenerate status and update commands | S | No |
| `crates/fornax-arw/README.md` | Sony ARW implementation | K,L,N,F | Merge/replace with `fornax-raw-formats` README | M | No |
| `crates/fornax-cli/README.md` | CLI usage | K,P,D,N,F | Fix decoder fallback claim and facade package path | M | Yes: `crates/fornax-cli/README.md:17` says Rawler fallback; code uses DNC/unsupported at `crates/fornax-decoder/src/lib.rs:45` |
| `crates/fornax-codec/README.md` | Codec contract | K,L,N,F | Rewrite neutral buffer, features, vendor functions, and disabled-codec errors | L | Yes: `crates/fornax-codec/README.md:6` says optional Zstandard; dependency is unconditional at `crates/fornax-codec/Cargo.toml:13` |
| `crates/fornax-core/README.md` | Core types | K,L,P,D,N,F | Reduce to neutral scope and move raw examples to raw-types | M | No |
| `crates/fornax-decoder/README.md` | Decoder dispatch | K,L,D,N,F | Replace with raw facade/raw-formats documentation | M | Yes: `crates/fornax-decoder/README.md:22` calls five methods `IDecoder`; `dng_metadata` is separate |
| `crates/fornax-devtool/README.md` | Development helpers | K,L | Split neutral and raw helper documentation | M | No |
| `crates/fornax-dng/README.md` | DNG reader/writer | K,L,N,F | Move raster semantics out and document shared metadata mapping | M | No |
| `crates/fornax-postprocessor/README.md` | Demosaic/color | K,L,P,D,F | Replace with raw-pipeline demosaic/color sections | L | No |
| `crates/fornax-preprocessor/README.md` | Normalization | K,L,P,F | Merge into raw-pipeline normalize section | S | Yes: `crates/fornax-preprocessor/README.md:15` names obsolete `DecodedRaw`/`RawImage` |
| `crates/fornax-py/README.md` | Python binding | K,L,F | Mark blocked; rewrite after binding API is rebuilt | M | Yes: `crates/fornax-py/README.md:3` claims a binding while `crates/fornax-py/src/lib.rs:10` imports absent facade APIs |
| `crates/fornax-tiff/README.md` | TIFF support | K,L,N,F | Rewrite DNG constant ownership, open enums, raster feature, and dependencies | L | No |
| `crates/fornax/README.md` | Current facade | K,P,D,N,F | Replace with `fornax-raw` README and compatibility guidance | M | No |
| `docs/architecture.md` | Main architecture | K,L,P,D,N,F | Rewrite around two workspaces and target graph | L | Yes: `docs/architecture.md:25` non-member crates; `docs/architecture.md:39` obsolete trait |
| `docs/decoder/architecture.md` | Decoder design | K,L,P,D,N,F | Rewrite formats, module map, traits, benches, and data flow | L | Yes: `docs/decoder/architecture.md:12` lists non-member crates; `docs/decoder/architecture.md:45` obsolete trait, while current dispatch is `crates/fornax-decoder/src/lib.rs:15` |
| `docs/decoder/benchmark.md` | Decoder benchmarks | K,L,N | Repoint decoder bench ownership to raw-formats/raw facade | M | No |
| `docs/decoder/corpus.md` | Corpus/regression design | K,L,N,F | Update package paths and facade references | M | No |
| `docs/decoder/formats/canon-cr3.md` | CR3 plan | P,D,N,F | Repoint planned implementation from decoder to raw-formats | S | No |
| `docs/decoder/formats/dng.md` | DNG plan | K,L,P,D,N,F | Update DNG reader/writer/tiff-raster boundaries | M | No |
| `docs/decoder/formats/fuji-raf.md` | RAF plan | P,D,N,F | Repoint future decoder crate | S | No |
| `docs/decoder/formats/hasselblad-3fr.md` | 3FR plan | P,D,N,F | Repoint future decoder crate | S | No |
| `docs/decoder/formats/nikon-nef.md` | NEF plan | P,D,N,F | Repoint future decoder crate | S | No |
| `docs/decoder/formats/other.md` | Unsupported-format policy | - | Update only if format dispatch terminology changes | S | No |
| `docs/decoder/formats/panasonic-rw2.md` | RW2 plan | P,D,N,F | Repoint future decoder crate | S | No |
| `docs/decoder/formats/phase-one-iiq.md` | IIQ plan | P,D,N,F | Repoint future decoder crate | S | No |
| `docs/decoder/formats/sony-arw.md` | Sony ARW plan | K,L,N,F | Move implementation ownership to raw-formats and update codec placement | M | No |
| `docs/plans/agents-md-revision.md` | Historical agent-doc plan | K,L,P,D,N | Keep as history; optionally prepend a superseded note | None | Historical |
| `docs/plans/amaze-implement.md` | Historical AMaZE plan | K,L,P,D,N,F | Keep historical paths or add a superseded note | None | Historical |
| `docs/plans/book-01-bootstrap.md` | Historical book plan | K,L,P,D,N | Keep as history | None | Historical |
| `docs/plans/book-02-figures.md` | Historical figure plan | K,L,P | Keep as history | None | Historical |
| `docs/plans/book-03-build.md` | Historical PDF plan | L | Keep as history | None | Historical |
| `docs/plans/chore-agent-guidance-refresh.md` | Historical guidance plan | K,L,P,D,N,F | Keep historical; avoid treating old crate claims as current | None | Historical |
| `docs/plans/chore-ci-change-detection.md` | Historical CI plan | L | Keep historical; real workflow must change separately | None | Historical |
| `docs/plans/chore-codec-enum-api.md` | Historical codec API plan | K,L,N | Keep historical; target codec API supersedes it | None | Historical |
| `docs/plans/chore-codec-writer-perf.md` | Historical codec/writer plan | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/chore-compare-demosaic-algorithm-plans.md` | Historical comparison plans | K,L,P,D,N | Keep historical; update only live comparison guide | None | Historical |
| `docs/plans/chore-decoder-benchmark-devtool.md` | Historical benchmark/devtool plan | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/chore-decoder-tracker-agent-guide.md` | Historical tracker guide plan | K,L,N | Keep historical | None | Historical |
| `docs/plans/chore-demosaic-parameter-audit.md` | Historical demosaic audit | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/chore-dng-compression-coverage.md` | Historical DNG compression plan | K,L,N,F | Keep historical; live compression reference must change | None | Historical |
| `docs/plans/chore-dng-raw-ifd-compat-performance.md` | Historical DNG IFD plan | K,L,P,D,N,F | Keep historical | None | Historical |
| `docs/plans/chore-dng-zstd-writer-bench.md` | Historical Zstd writer plan | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/chore-extract-fornax-codec.md` | Historical codec extraction | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/chore-fornax-cli.md` | Historical CLI plan | K,L,P,D,N,F | Keep historical; live CLI docs change | None | Historical |
| `docs/plans/chore-fornax-decoder-scaffold.md` | Historical decoder scaffold | K,L,P,D,N,F | Keep historical | None | Historical |
| `docs/plans/chore-merge-bench-crates.md` | Historical benchmark merge | K,L,P,D | Keep historical | None | Historical |
| `docs/plans/chore-move-demosaic-protocol-types.md` | Historical protocol move | K,L,P,D,F | Keep historical; target moves protocols again | None | Historical |
| `docs/plans/chore-pipeline-preview-color.md` | Historical color pipeline | K,L,P,D,N,F | Keep historical | None | Historical |
| `docs/plans/chore-rawtherapee-demosaic-reference.md` | Historical reference integration | K,L,P,D,F | Keep historical | None | Historical |
| `docs/plans/chore-simplify-codec-dependencies.md` | Historical dependency plan | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/chore-tiff-dng-crates.md` | Historical TIFF/DNG split | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/dcb-faithful-port.md` | Historical DCB port | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/dec-arw-sony-tiff.md` | Historical ARW plan | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/dec-corpus-lfs-tracker.md` | Historical corpus tracker | K,L,N,F | Keep historical | None | Historical |
| `docs/plans/dec-dng-roundtrip.md` | Historical DNG migration | K,L,P,D,N,F | Keep historical | None | Historical |
| `docs/plans/dht-faithful-port.md` | Historical DHT port | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/lmmse-faithful-port.md` | Historical LMMSE port | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/vng-faithful-port.md` | Historical VNG port | K,L,P,D,N | Keep historical | None | Historical |
| `docs/plans/vng4-native.md` | Historical VNG4 plan | K,L,P,D | Keep historical | None | Historical |
| `docs/references/aahd.md` | AAHD research | D,N | No path rewrite unless source links are added | None | No |
| `docs/references/amaze.md` | AMaZE research | L,D,N,F | No content rewrite expected | None | No |
| `docs/references/dcb.md` | DCB research | K,D,N,F | No content rewrite expected | None | No |
| `docs/references/dht.md` | DHT research | K,P,D,N | No content rewrite expected | None | No |
| `docs/references/dng-compression.md` | Compression references | N,F | Split generic TIFF/DNG origins and update target ownership notes | M | No; origin gaps are separate evidence gaps |
| `docs/references/dng-lossless-jpeg.md` | DNG/LJPEG references | K,L,N,F | Update implementation links and constant ownership | S | No |
| `docs/references/dng-preview-color.md` | Preview/color references | D,N,F | Update pipeline module links | S | No |
| `docs/references/lmmse.md` | LMMSE research | D,N | No content rewrite expected | None | No |
| `docs/references/rawtherapee.md` | RawTherapee references | K,P,D,N,F | Update live implementation links | S | No |
| `docs/references/sony-arw.md` | Sony research | L,N | Update implementation links from ARW to raw-formats | S | No |
| `docs/references/vng.md` | VNG research | K,D,N | No content rewrite expected | None | No |
| `docs/superpowers/specs/2026-11-03-dcb-faithful-port-design.md` | DCB design record | K,L,D,N | Keep as historical design record | None | Historical |
| `docs/superpowers/specs/2026-11-04-dht-faithful-port-design.md` | DHT design record | K,D,N | Keep as historical design record | None | Historical |

## Cross-cutting documentation work

`[TARGET]` Live architecture documentation should be rewritten from the final
crate graph after Steps 3, 4, 5, 8, and 9, not incrementally from this
proposal's intermediate names.

`[TARGET]` Historical plans and superpowers design records should retain their
original crate names; changing them would falsify the decision record.

`[TARGET]` Generated comparison and tracker tables should be regenerated with
their existing tools after package paths stabilize, then reviewed as normal
generated output.
