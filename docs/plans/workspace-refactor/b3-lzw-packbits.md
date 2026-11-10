# LZW and PackBits Codec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add feature-gated TIFF LZW and PackBits decoding/encoding plus the legacy Deflate identifier.

**Architecture:** `fornax-codec` owns both algorithms behind `lzw` and `packbits`; `fornax-tiff` forwards those features and resolves TIFF IDs only when the corresponding feature is enabled. LZW uses an established crate after its API, license, and dependency impact are verified; PackBits remains a small in-tree byte decoder/encoder.

**Tech Stack:** Rust 2024, Cargo features, `weezl` candidate for TIFF LZW.

**Spec:** `docs/plans/workspace-refactor/phase-b-c-handoff.md`

## Global Constraints

- Use branch `chore/codec-lzw-packbits` and worktree
  `.worktrees/chore-codec-lzw-packbits`.
- Use one worktree and one pull request.
- Keep the branch plan at `docs/plans/chore-codec-lzw-packbits.md`.
- Start after B1 and B2 are merged.
- Add only `lzw` and `packbits`; do not add LZMA, JPEG, WebP, or vendor codecs.
- Do not place TIFF or raw-workflow types in `fornax-codec`.
- Do not enable the new codecs in `fornax-dng`; DNG decoding results must not change.
- Do not change crate names, project versions, vendor codec placement, or workspace structure.
- Do not compile `compare-demosaic`.

## Review Focus

- A disabled codec feature must produce the B2 typed error naming that feature.
- TIFF LZW bit order and code-width growth must interoperate with the fixed known vector.
- Horizontal prediction must be reversed after LZW decompression and applied before LZW compression.
- PackBits repeated, literal, and no-op control bytes must decode to the exact byte stream.
- Default `fornax-codec` dependency output must not gain an unconditional LZW dependency.

---

### Task 1: Establish the LZW dependency and test contract

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/fornax-codec/Cargo.toml`
- Create: `crates/fornax-codec/src/lzw.rs`

**Interfaces:**
- Consumes: `DecodeRequest`, `EncodeRequest`, `SampleBuffer`, `predictor`, and the existing codec error helpers.
- Produces: optional codec feature `lzw` and modules implementing the existing codec dispatch signature:

```rust
pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError>;
pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
```

- [ ] **Step 1: Verify the LZW dependency**

Use the already locked `weezl` version as the candidate. Verify its current
license, maintenance metadata, and dependency list from crates.io and Cargo
metadata. Confirm the API supports TIFF LZW's MSB-first codes and TIFF code-size
switch behavior. If it does not, stop and report the mismatch before choosing
another crate.

Record in the PR:

- selected crate and version;
- license;
- direct and transitive dependency count;
- why it is preferable to an in-tree LZW implementation.

- [ ] **Step 2: Add the optional dependency**

In the root workspace dependencies, add the locked `weezl` version. In
`crates/fornax-codec/Cargo.toml`, add:

```toml
weezl = { workspace = true, optional = true }

[features]
lzw = ["dep:weezl"]
```

Keep existing features unchanged.

- [ ] **Step 3: Write failing LZW tests**

Add module tests in `crates/fornax-codec/src/lzw.rs` for:

```rust
const TIFF_LZW_HEX: &[u8] = &[
    0x80, 0x0b, 0x60, 0x50, 0x22, 0x0c, 0x0c, 0x85, 0x01,
];
const TIFF_LZW_DECODED: &[u8] = b"-----A---B";
```

The first test decodes `TIFF_LZW_HEX` as a 9x1 U8 image and asserts
`TIFF_LZW_DECODED`. The second encodes `TIFF_LZW_DECODED` as LZW, decodes it,
and asserts round-trip equality.

Run:

```bash
cargo test -p fornax-codec --features lzw lzw
```

Expected: FAIL because the LZW dispatch and implementation are incomplete.

- [ ] **Step 4: Implement TIFF LZW decode/encode**

Follow the established chunk loop used by Deflate: validate geometry, decode
each chunk, reverse the requested predictor, and join payloads through
`decode_payloads`. For encoding, serialize samples, apply the predictor, and
encode each chunk independently.

Map malformed streams to `CodecError::InvalidInput` with the chunk index.
Reject unsupported predictor combinations through the shared predictor helpers.

- [ ] **Step 5: Test horizontal predictor interplay**

Add a test that encodes and decodes a 4x1 U8 row `[10, 12, 13, 15]` with
`DecodeCompression::Lzw`, `EncodeCompression::Lzw`, and
`Predictor::HorizontalInteger`. Assert the decoded row equals the input.

- [ ] **Step 6: Run LZW tests**

```bash
cargo test -p fornax-codec --features lzw lzw
```

Expected: PASS, including the fixed interoperability vector and predictor case.

### Task 2: Add PackBits and codec dispatch

**Files:**
- Create: `crates/fornax-codec/src/packbits.rs`
- Modify: `crates/fornax-codec/src/request.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `crates/fornax-codec/README.md`

**Interfaces:**
- Consumes: the Task 1 module pattern.
- Produces: optional `DecodeCompression::Lzw`, `EncodeCompression::Lzw`,
  `DecodeCompression::PackBits`, and `EncodeCompression::PackBits` variants.

- [ ] **Step 1: Add PackBits failing tests**

Test `[0xfe, 0xaa, 0x02, 1, 2, 3]` as a 6x1 U8 image decoding to
`[0xaa, 0xaa, 0xaa, 1, 2, 3]`. Add a literal-only case and a round-trip case.

Run:

```bash
cargo test -p fornax-codec --features packbits packbits
```

Expected: FAIL because PackBits does not exist.

- [ ] **Step 2: Implement PackBits**

Implement the TIFF PackBits control-byte rules without a new dependency:

- `0..=127` copies the following `control + 1` literal bytes;
- `-127..=-1` repeats the following byte `1 - control` times;
- `-128` is a no-op.

Reject truncated controls, literal runs, or repeated-byte runs with a chunk
index in the error. Use the same chunk geometry, predictor, and payload joining
path as LZW.

- [ ] **Step 3: Gate public variants and dispatch**

Add feature-gated variants to both compression enums and matching `cfg` arms
in `decode` and `encode`.

- [ ] **Step 4: Run codec feature matrix**

```bash
cargo test -p fornax-codec
cargo test -p fornax-codec --features lzw
cargo test -p fornax-codec --features packbits
cargo test -p fornax-codec --all-features
```

Expected: all commands exit 0.

- [ ] **Step 5: Commit codec changes**

```bash
git add Cargo.toml Cargo.lock crates/fornax-codec
git commit -m "feat: add optional LZW and PackBits codecs"
```

### Task 3: Wire TIFF resolver features

**Files:**
- Modify: `crates/fornax-tiff/Cargo.toml`
- Modify: `crates/fornax-tiff/src/raster.rs`
- Modify: `crates/fornax-tiff/tests/raster.rs`
- Modify: `crates/fornax-tiff/README.md`

**Interfaces:**
- Consumes: B2's `TiffCompressionError` and resolver signature.
- Produces:

```toml
lzw = ["raster", "fornax-codec/lzw"]
packbits = ["raster", "fornax-codec/packbits"]
```

- [ ] **Step 1: Add resolver tests**

With all features, assert:

- `5` resolves to `DecodeCompression::Lzw`;
- `32773` resolves to `DecodeCompression::PackBits`;
- `32946` resolves to `DecodeCompression::Deflate`.

With only `raster`, assert `5` and `32773` return
`UnsupportedCompression` naming `lzw` and `packbits` respectively.

- [ ] **Step 2: Add feature forwards and resolver arms**

Add the two TIFF features and `cfg`-gated resolver arms. Keep IDs `34892` and
`52546` outside this resolver.

- [ ] **Step 3: Run TIFF feature tests**

```bash
cargo test -p fornax-tiff --no-default-features
cargo test -p fornax-tiff --no-default-features --features raster
cargo test -p fornax-tiff --all-features
```

Expected: all commands exit 0.

- [ ] **Step 4: Commit TIFF wiring**

```bash
git add crates/fornax-tiff
git commit -m "feat: resolve optional TIFF LZW and PackBits"
```

### Task 4: Full validation and handoff

**Files:**
- Modify: `docs/plans/chore-codec-lzw-packbits.md`

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: verification evidence.

- [ ] **Step 1: Run repository validation**

```bash
scripts/agent-validation.sh
```

Expected: exit 0.

- [ ] **Step 2: Check dependency trees**

```bash
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-codec -e normal --depth 1 --features lzw,packbits
cargo tree -p fornax-tiff -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1 --features raster,lzw,packbits
```

Expected: `weezl` appears only for enabled LZW builds; default TIFF still has
no codec.

- [ ] **Step 3: Re-run golden hashes**

```bash
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: `golden: unchanged (9 rows)`.

- [ ] **Step 4: Record dependency/license evidence**

Add the selected crate, version, license, dependency count, test vectors, and
all command outcomes to the branch plan before opening the PR.
