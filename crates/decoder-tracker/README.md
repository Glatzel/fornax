# decoder-tracker

<!-- decoder-tracker-badge:start -->

[![RAW corpus: 395/2020 passed](https://img.shields.io/badge/raw.pixls.us_corpus_pass-395%2F2020-orange)](baseline.csv)

| Vendor | Format | Status |
| --- | --- | ---: |
| Adobe DNG Converter | DNG | 2/6 |
| Apple | DNG | 5/6 |
| Arashi Vision | DNG | 1/1 |
| Arri | ARI | 0/1 |
| Autel | DNG | 0/1 |
| Autel Robotics | DNG | 1/1 |
| Blackmagic | DNG | 2/9 |
| Blackmagic Design | DNG | 1/1 |
| Canon | CR2 | 0/142 |
| Canon | CR3 | 0/110 |
| Canon | CRW | 0/51 |
| Canon | DNG | 50/53 |
| Canon | TIF | 0/3 |
| Casio | CAM | 0/2 |
| DJI | DNG | 6/8 |
| Epson | ERF | 0/3 |
| Eyedeas | DNG | 1/1 |
| FIMI | DNG | 0/1 |
| FUJIFILM | RAF | 0/25 |
| Fujifilm | DNG | 0/1 |
| Fujifilm | RAF | 0/126 |
| GITUP | RAW | 0/4 |
| Gitup | RAW | 0/1 |
| GoPro | GPR | 0/17 |
| Google | DNG | 7/7 |
| HASSELBLAD | ARW | 1/1 |
| HMD Global | DNG | 1/1 |
| HTC | DNG | 1/1 |
| HUAWEI | DNG | 18/18 |
| Hasselblad | 3FR | 0/11 |
| Hasselblad | DNG | 1/2 |
| Hasselblad | FFF | 0/11 |
| ImBack | RAW | 0/1 |
| KONICA MINOLTA | MRW | 0/2 |
| KanDao | DNG | 2/3 |
| Kandao | DNG | 1/1 |
| Kodak | DCR | 0/5 |
| Kodak | KDC | 0/9 |
| Kodak | RAW | 0/11 |
| Kodak | TIF | 0/5 |
| LEICA CAMERA AG | DNG | 1/1 |
| LG | DNG | 8/9 |
| Leaf | IIQ | 0/4 |
| Leaf | MOS | 0/4 |
| Leica | DNG | 9/13 |
| Leica | RAW | 0/4 |
| Leica | RWL | 0/20 |
| Leica Camera AG | DNG | 1/1 |
| Light | DNG | 0/1 |
| Light | LRI | 0/1 |
| MADV | DNG | 1/1 |
| MINOLTA | MRW | 0/1 |
| Mamiya | MEF | 0/1 |
| Microsoft | DNG | 1/1 |
| Minolta | MDC | 0/1 |
| Minolta | MRW | 0/10 |
| Minolta Co., Ltd. | MRW | 0/1 |
| NIKON CORPORATION | NEF | 0/42 |
| NIKON CORPORATION | NRW | 0/1 |
| Nikon | DNG | 1/2 |
| Nikon | NEF | 0/235 |
| Nikon | NRW | 0/11 |
| Nokia | DNG | 2/2 |
| OLYMPUS | ORF | 0/2 |
| OLYMPUS IMAGING CORP. | ORF | 0/1 |
| OM Digital Solutions | ORF | 0/8 |
| OM Digital Solutions | ORI | 0/6 |
| OM System | ORF | 0/12 |
| OM System | ORI | 0/8 |
| Olympus | ORF | 0/70 |
| Olympus | ORI | 0/5 |
| OnePlus | DNG | 4/4 |
| PARROT | DNG | 0/2 |
| PENTAX | DNG | 1/1 |
| PENTAX | PEF | 0/1 |
| Panasonic | DNG | 1/1 |
| Panasonic | RAW | 0/10 |
| Panasonic | RW2 | 0/402 |
| Paralenz | RAW | 0/2 |
| Parrot | DNG | 1/1 |
| Pentax | DNG | 34/35 |
| Pentax | PEF | 0/40 |
| Pentax | RAW | 0/1 |
| Phase One | IIQ | 0/28 |
| Phase One | TIF | 0/12 |
| Phase One AS | IIQ | 0/3 |
| Plustek | DNG | 0/2 |
| Polaroid | X3F | 0/1 |
| RICOH | DNG | 2/2 |
| RaspberryPi | DNG | 3/3 |
| RaspberryPi | RAW | 0/7 |
| Realme | DNG | 1/1 |
| Ricoh | DNG | 5/5 |
| SAMSUNG TECHWIN | PEF | 0/1 |
| SJCAM | RAW | 0/4 |
| SJCam | RAW | 0/2 |
| SONY | ARW | 24/31 |
| Samsung | DNG | 13/22 |
| Samsung | SRW | 0/27 |
| Sigma | DNG | 8/9 |
| Sigma | X3F | 0/17 |
| Sinarback | STI | 0/1 |
| Sony | ARQ | 0/1 |
| Sony | ARW | 161/174 |
| Sony | SR2 | 0/1 |
| Sony | SRF | 0/1 |
| XIAOYI | DNG | 0/3 |
| Xiaomi | DNG | 5/7 |
| Xiaomi | RAW | 0/1 |
| Xiro | RAW | 0/1 |
| YI TECHNOLOGY | DNG | 1/1 |
| Yuneec | DNG | 0/2 |
| asus | DNG | 2/2 |
| bq | DNG | 1/1 |
| moto g8plus | DNG | 0/1 |
| motorola | DNG | 3/3 |
| samsung | DNG | 0/1 |

<!-- decoder-tracker-badge:end -->

Manual regression tracking for Fornax decoders over the pinned raw.pixls.us
corpus.

Maintainers and agents should read the [decoder-tracker agent guide](AGENTS.md)
before initializing the corpus, updating the baseline, or regenerating the
README.

The corpus submodule is checked out as Git LFS pointer metadata. `run` fetches
only the selected shard's payloads, executes each file in an isolated child with
a 300-second timeout, and writes a canonical no-timings CSV. Full local runs
require `--allow-full` or `CI=true`; the default local guard permits two files.

```bash
pixi run -e dev cargo run -p decoder-tracker -- manifest --check
pixi run -e dev cargo run -p decoder-tracker -- select --glob 'Canon/**/*.DNG'
pixi run -e dev cargo run -p decoder-tracker -- run --filename IMG_0001
pixi run -e dev cargo run -p decoder-tracker -- merge --manifest MANIFEST SHARD...
pixi run -e dev cargo run -p decoder-tracker -- diff --selected MANIFEST BASELINE CANDIDATE
pixi run -e dev cargo run -p decoder-tracker -- update --baseline BASELINE --candidate CANDIDATE
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
```

The committed corpus metadata manifest is `manifest.csv`; decoder pass columns
and outcomes are in `baseline.csv`. DuckDB CLI owns plan-time manifest filtering
and the shell-driven merge, selected-key diff, and baseline upsert path. Stage
columns remain numeric for SQL; a leading emoji
`status` column makes pass/fail visually scannable. `update-badge.sh` joins the
full manifest to the baseline and writes the accumulated badge and
per-vendor-format `passed/total` table between the README markers;
vendor-format pairs without baseline rows report `0/total`. The complete
schema, stage semantics, glob/sharding rules, workflow, and update procedure
are documented in
[docs/decoder/corpus.md](../../docs/decoder/corpus.md).
