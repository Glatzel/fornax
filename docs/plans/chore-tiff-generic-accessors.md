# Generic TIFF Accessors Implementation Plan

**Goal:** Share only the generic unsigned TIFF scalar/list access behavior used
by `fornax-dng` and `fornax-arw`, without moving DNG or Sony policy.

**Architecture:** `fornax-tiff` will expose one scalar accessor and one typed
unsigned-vector accessor. DNG and ARW will retain thin adapters that translate
generic access errors into each crate's existing `DecodeError` messages, so
caller behavior remains unchanged.

## Scope

**In scope**

- Add generic unsigned scalar/list conversion and directory vector access to
  `fornax-tiff`.
- Add focused public-API tests for accepted scalar/list values, scalar/list
  selection, target-width overflow, missing tags, and rejected value types.
- Replace the equivalent DNG and ARW helpers with the shared API.
- Preserve the current DNG and ARW user-facing error categories and detail
  messages.
- Document the exact shared behavior and intentional behavior differences in
  this plan and the pull request.

**Out of scope**

- Moving `active_area`, `default_crop`, `crop_and_cast`, or Sony model/crop
  fallbacks.
- Moving DNG `unsigned_values` / `scalar_number`, which intentionally accept
  broader numeric types.
- Sharing ARW byte-list flattening or DNG opcode-byte validation, which have
  different accepted types and validation semantics.
- Moving strip/tile layout, DNG constants, codec buffers, or starting Step 4.
- Modifying `TODO.md`.

## Exact Behavior to Preserve

- `unsigned_scalar` accepts `Byte`, `Short`, `Unsigned`, `Ifd`,
  `UnsignedBig`, and `IfdBig`.
- For nested `List` values, `unsigned_scalar` recursively takes the first
  value; a missing or empty list returns `None`.
- `unsigned_scalar` rejects signed, floating-point, rational, and ASCII
  scalar types even when their numeric value is non-negative.
- `unsigned_vec` accepts a scalar as a one-element vector and a flat list in
  order; an empty list produces an empty vector.
- `unsigned_vec` delegates list conversion to `Value::into_u64_vec`, so
  non-negative signed integer variants remain accepted. Negative signed
  values, rationals, floating-point values, nested lists, and other
  non-convertible variants are invalid.
- `unsigned_vec` distinguishes a missing tag, an invalid value type, and a
  value that does not fit the requested target integer type.
- Existing callers request `usize`; DNG and ARW only map the shared error
  variants back to their prior `DecodeError::InvalidInput` messages.

## Task Checklist

- [x] **Step 1: Write failing TIFF accessor tests**

  Add `crates/fornax-tiff/tests/value_accessors.rs` covering accepted scalar
  types, recursive first-list scalar selection, scalar rejection, vector
  scalar/list handling, missing tags, rejected vector value types, and `u8`
  target overflow from a TIFF `LONG` value.

- [x] **Step 2: Verify RED**

  Run `cargo test -p fornax-tiff --test value_accessors` and confirm the tests
  fail because the shared accessors do not exist. Result: compilation failed
  with unresolved `unsigned_scalar`, `unsigned_vec`, and `ValueAccessError`
  imports as expected.

- [x] **Step 3: Implement the shared API**

  Add `unsigned_scalar`, generic `unsigned_vec`, and `ValueAccessError` to
  `fornax-tiff`, re-exporting them from the crate root.

- [x] **Step 4: Verify GREEN for TIFF**

  Run `cargo test -p fornax-tiff` and confirm the new tests and existing TIFF
  tests pass. Result: 18 tests passed across 6 suites.

- [x] **Step 5: Migrate DNG**

  Delete the duplicate DNG scalar helper, use the TIFF scalar accessor for DNG
  IFD selection and decoder metadata, and reduce DNG `unsigned_vec` to an
  error-message adapter over the TIFF accessor.

- [x] **Step 6: Migrate ARW**

  Delete the duplicate ARW scalar helper, use the TIFF scalar accessor in ARW
  container/metadata paths, and reduce ARW `unsigned_vec` to an error-message
  adapter over the TIFF accessor.

- [x] **Step 7: Verify caller behavior**

  Run `cargo test -p fornax-dng` and `cargo test -p fornax-arw`; confirm both
  crates retain their existing tests and diagnostics. Result: DNG passed 28
  tests across 3 suites and ARW passed 11 tests across 2 suites.

- [x] **Step 8: Run repository validation**

  Run the requested focused tests, workspace test suite, agent validation,
  golden decode hash check, and `git diff --check`. Result:
  `cargo test --workspace --all-features --exclude compare-demosaic` passed
  270 tests across 42 suites; `scripts/agent-validation.sh` exited successfully;
  and the decoder golden check reported `golden: unchanged (9 rows)`.

- [ ] **Step 9: Record outcomes and open the PR**

  Update this plan with validation results and decisions, inspect the final
  diff, commit the change, push the branch, and open one PR titled
  `refactor: share generic TIFF accessors`.

## Files Expected to Change

- `docs/plans/chore-tiff-generic-accessors.md`
- `crates/fornax-tiff/src/lib.rs`
- `crates/fornax-tiff/src/value.rs`
- `crates/fornax-tiff/tests/value_accessors.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/dng_decoder/transforms.rs`
- `crates/fornax-arw/src/container.rs`
- `crates/fornax-arw/src/metadata.rs`

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Decisions and Notes

- **Shared API shape:** A scalar free function preserves the existing
  `Option<&Value> -> Option<u64>` call shape. A typed vector accessor accepts
  any `TryFrom<u64>` target, allowing the callers to request `usize` while
  making overflow behavior directly testable on 64-bit CI with `u8`.
- **Error ownership:** `fornax-tiff` reports generic access failures only.
  DNG and ARW retain local adapters because their existing diagnostics and
  error variants belong to those format crates.
- **Intentional type asymmetry:** Scalar access remains unsigned-only while
  list conversion continues through `Value::into_u64_vec`, matching current
  DNG and ARW behavior rather than standardizing one acceptance policy.
- **Byte helpers stay local:** ARW recursively flattens byte lists; DNG accepts
  only a byte scalar or a one-level byte list for opcode data. Sharing either
  would change accepted values or error behavior.
- **Open refactor decisions:** Neutral buffer shape, vendor codec placement,
  and crate naming remain untouched.
- **Validation note:** The first agent-validation run completed all tests and
  doctests but failed on an unused `Value` import in DNG. The import was
  removed, nightly formatting was applied, and the complete validation script
  passed on rerun.
