# XERJ trial: NEF Huffman row-kernel design

Analysis-only trial run 2026-11-17 against Fornax `b98c966`. No codec code
changed. Task A inventory, classification, and the old-rule map are in
`docs/plans/chore-agent-context-audit.md`.

## B1 Setup

- Checked `xerj --version` (`v1.0.0-rc.100`), then `curl` to localhost:9200.
  No node was running, so I started the installed binary without changing its
  configuration.
- Bind was `127.0.0.1` only for REST/ES/gRPC; API-key auth was on. Data lived at
  `/home/glatzel/.xerj-trial-fornax-2026-11`, outside every indexed root.
- Indexed only staged hard links for `fornax-codec`, `fornax-tiff`,
  `fornax-raw-core`, `fornax-dng`, `fornax-raw-decoder`, `fornax-raw`, all
  seven vendor decoder crates, FlashRaw 0.3.1, and LibRaw.
- FlashRaw was fetched at package commit
  `61c32fb1abbc9e819084e01e62703ceb4c4998db`, licence MIT/Apache-2.0. LibRaw
  was `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, licence
  LGPL-2.1/CDDL-1.0.
- `code-and-docs-no-vectors`, no semantic text, no graph. Indexing took
  **22.869 s**: 1,147 files seen, 967 unique contents indexed, 40 duplicates,
  140 junk/binary files, 656 code files, and **31,092 records**. Data was 65 MB.
- RAW extensions, test fixtures, `target/`, and hidden repositories were
  excluded. A no-op reconciliation took **0.848 s**; an actual one-file edit
  was not made because the task required no source changes.

## B2 Design note

- FlashRaw builds a canonical direct table, then a 13-bit fused table carrying
  signed difference plus code/difference bit count; escapes share an exact
  fallback (`external/flashraw/crates/flashraw-codecs/src/nikon_huffman.rs:25`,
  `:48`, `:71`).
- Its scalar row path keeps one 64-bit MSB window, decodes four samples per
  load, and switches to an exact reader when the conservative row bound cannot
  prove enough input remains (`:282`, `:315`, `:383`).
- LibRaw builds six static count/symbol trees, loads four initial predictors,
  decodes `len`/`shl` differences, updates two vertical then two horizontal
  predictors, and maps through the curve
  (`external/libraw/src/decoders/decoders_dcraw.cpp:873`, `:901`, `:932`,
  `:936`, `:941`).
- FlashRaw parallelizes only one-table streams: speculative chunk walks,
  sequential symbol-boundary stitching, then row-band predictor rebuild
  (`nikon_huffman.rs:424`, `:473`, `:512`; `par_entropy.rs:12`). LibRaw is
  serial in this path.
- Fornax plan: add a validated `FastHuffmanTable` and 64-bit window API in
  `crates/fornax-codec/src/huffman.rs`; keep Nikon tables, predictor policy,
  and curve mapping in `crates/fornax-raw-nikon/src/codec.rs`; bridge exports
  through `crates/fornax-raw-nikon/src/huffman.rs`; extend generic and NEF
  tests for escapes, tails, both row parities, odd widths, curve/overflow
  semantics, and randomized equivalence. Ship scalar fused rows first.

## B3 A/B measurements

| Metric | Run 1 baseline | Run 2 XERJ |
| --- | ---: | ---: |
| Tools | normal `rg`/`sed` | `xerj search`/`def` |
| Core task wall time | 46 s | 61 s |
| Tool calls | 30 | 16 retrieval calls plus `gain` |
| Full source files opened | 10 | 0 |
| Explicit source bytes loaded | ~74.7 KB | 0 |
| Harness tool-result tokens | ~27.7 k | ~28.5 k |
| Correction/retry steps | 1 | 2 |

The harness does not expose total model input/output tokens. Token figures above
are the per-call result-token counts summed for filesystem/XERJ output; they are
not a complete conversation-token count. XERJ's own `gain` reported 20 searches,
100% had hits, p50 51 ms, p95 165 ms. This XERJ version did not report a token
savings figure, so there was no usable tool-provided savings estimate.

## B4 Quality check

- Run 1 was verified at FlashRaw fused table/bit-window lines `25`, `48`, `282`,
  `315`; LibRaw `len`/`shl` and predictor lines `932`, `936`, `941`; Fornax
  lookup/read/checked-add lines `crates/fornax-codec/src/huffman.rs:25`,
  `:268` and `crates/fornax-raw-nikon/src/codec.rs:123`.
- Run 2 was verified at `par_entropy.rs:108`/`:110`/`:112`,
  `nikon_huffman.rs:473`, and Fornax `huffman.rs:25`/`:175`/`:268`.
- Run 1 missed a useful detail Run 2 surfaced: exact 256 KiB stream, 128 KiB
  chunk, and four-tasks-per-thread thresholds.
- Run 2 missed Run 1's exact LibRaw `shl` expression and row cancellation
  placement. XERJ also displayed wrong locators: `decode_huffman` was reported
  at 96/110 but is at `codec.rs:61`; one `nikon_load_raw` hit said 932 while
  the function is at `decoders_dcraw.cpp:871`.
- Net quality is not equal or better. The designs agree on the main kernel, but
  Run 2 requires source re-verification and has one material retrieval gap.
  Therefore its slightly worse token result is not a savings signal.

## B5 Verdict

Setup is tolerable for an already-running node, but this trial cost more wall
time and tool-output tokens, produced no qualifying quality gain, required a
staging workaround for hidden-worktree paths, and exposed line-locator trust
issues. The node itself stayed loopback-only and authenticated; cleartext local
transport and no engine encryption-at-rest remain deployment caveats.
Recommendation: **drop** XERJ for general Fornax work and for this codec
kernel task. Keep the existing conservative `AGENTS.md` line that limits XERJ
to unfamiliar external reference lookup and requires `file:line` verification;
do not add an adoption line. No XERJ feedback PR was opened.
