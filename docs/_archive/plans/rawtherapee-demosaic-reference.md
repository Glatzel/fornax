# RawTherapee demosaic reference integration

This archived plan documents the completed RawTherapee demosaic reference integration work.

## Goal

Add RawTherapee 5.13 as a pinned comparison reference and exercise every Bayer
demosaic method that exists in both RawTherapee and Fornax. Prepare reference
adapters for IGV, EAHD, and HPHD and track their Fornax ports in `TODO.md`.
Dual demosaic variants are explicitly excluded.

## Scope

In scope:

- Conventional 2x2 RGB Bayer input only.
- RawTherapee source provisioning, adapters, backend dispatch, metadata, and
  tests.
- Comparisons for AHD, AMaZE, DCB, LMMSE, RCD, and VNG4.
- Direct adapter tests for IGV, EAHD, and HPHD.
- Reference notes, baseline, generated README, and `TODO.md` updates.

Out of scope:

- Fornax implementations of IGV, EAHD, or HPHD.
- Dual, AMaZE+Bilinear, AMaZE+VNG4, RCD+Bilinear, RCD+VNG4, DCB+Bilinear, or
  DCB+VNG4.
- X-Trans, four-color, or monochrome demosaicing.
- RawTherapee decoding, preprocessing, white balance, or output-color stages.

## Task checklist

- [x] **Task 1:** Add backend metadata and reference-selection tests.
- [x] **Task 2:** Add pinned source provisioning and RawTherapee adapters.
- [x] **Task 3:** Register shared-algorithm comparisons.
- [x] **Task 4:** Add reference documentation and TODO entries.
- [x] **Task 5:** Refresh generated artifacts and run full validation.
- [x] Pin RawTherapee release 5.13 at commit
      `1924179107995b01b73cdc7f4fe91e44fc8227a7` in the platform setup scripts and
      `build.rs`.
- [x] Add a RawTherapee compatibility facade and adapters for AHD, AMaZE, DCB,
      LMMSE, RCD, VNG4, IGV, EAHD, and HPHD.
- [x] Add the RawTherapee backend and stable algorithm IDs to the C++ shim and
      Rust FFI dispatch.
- [x] Register RawTherapee cases for every shared Fornax/RawTherapee algorithm.
- [x] Add direct reference tests and checksum snapshots for IGV, EAHD, and HPHD.
- [x] Add RawTherapee reference and version metadata to canonical JSON and the
      generated README.
- [x] Document RawTherapee source, parameters, compatibility differences, and
      GPL-3.0-or-later constraints.
- [x] Add `[IGV]`, `[EAHD]`, and `[HPHD]` to `TODO.md`; update `[AMAZE]` to
      include both darktable and RawTherapee.
- [x] Refresh snapshots, the canonical baseline, and generated README regions.
- [x] Run all required validation and inspect the final diff.

## Task 1: Backend metadata and selection

**Files:** `crates/compare-demosaic/src/cases.rs`,
`crates/compare-demosaic/src/reference.rs`, `crates/compare-demosaic/src/run.rs`,
`crates/compare-demosaic/src/report.rs`, `crates/compare-demosaic/src/lib.rs`,
`crates/compare-demosaic/src/main.rs`, `crates/compare-demosaic/cpp/shim.{h,cpp}`,
`crates/compare-demosaic/build.rs`.

**Steps:**

1. Add a failing test asserting `ReferenceBackend::RawTherapee.library()`,
   `reference_names()`, and the source-link version map recognize
   `rawtherapee`.
2. Run `cargo test -p compare-demosaic source_links_match_packaged_versions`;
   expected: compile failure because `ReferenceBackend::RawTherapee` and the
   RawTherapee metadata do not exist.
3. Add RawTherapee version/commit constants, report metadata, backend dispatch,
   and FFI backend ID without registering cases yet.
4. Run the test; expected: pass.

## Task 2: Pinned sources and adapters

**Files:** `scripts/setup.{sh,ps1}`,
`crates/compare-demosaic/cpp/algos/rawtherapee*/**`,
`crates/compare-demosaic/cpp/algos/rawtherapee_*.{cpp,c}`,
`crates/compare-demosaic/cpp/shim.{h,cpp}`, `crates/compare-demosaic/build.rs`,
`crates/compare-demosaic/src/lib.rs`.

**Steps:**

1. Add failing direct-reference tests for IGV, EAHD, and HPHD using a valid
   RawTherapee algorithm ID on the 128x128 synthetic fixture. Assert non-empty
   RGB output, deterministic second output, and retained sampled Bayer codes.
2. Run `cargo test -p compare-demosaic rawtherapee`; expected: fail because the
   RawTherapee adapters and IDs are absent.
3. Materialize RawTherapee 5.13 source files through both setup scripts and add
   the compatibility facade plus adapter sources for AHD, AMaZE, DCB, LMMSE,
   RCD, VNG4, IGV, EAHD, and HPHD. Normalize split-green masks, use identity
   color matrices, scalar paths, and RawTherapee's 16-bit working scale.
4. Wire all adapter sources into `build.rs` and the shim.
5. Run `cargo test -p compare-demosaic rawtherapee`; expected: pass.

## Task 3: Shared algorithm comparisons

**Files:** `crates/compare-demosaic/src/cases.rs`,
`crates/compare-demosaic/src/lib.rs`,
`crates/compare-demosaic/src/snapshots/**`.

**Steps:**

1. Add a failing test asserting RawTherapee cases exist for AHD, AMaZE, DCB,
   LMMSE, RCD, and VNG4 and that no algorithm ID or case is named Dual.
2. Run the test; expected: fail because the six cases are not registered.
3. Register the six cases with commit-bearing source links and the existing
   Fornax runners. Use DCB `(0, true)`, LMMSE `(1)`, and comparison-safe tiling
   for AMaZE/RCD.
4. Run the test and the comparison crate suite; expected: pass aside from
   intentionally obsolete snapshots, which are reviewed and updated.

## Task 4: Documentation and backlog

**Files:** `TODO.md`, `docs/reference/algorithms/rawtherapee.md`,
`crates/compare-demosaic/AGENTS.md`, `crates/compare-demosaic/README.md`.

**Steps:**

1. Update hand-written README notes and reference-source setup guidance.
2. Document the pinned source, GPL-3.0-or-later constraint, compatibility
   facade, identity-matrix assumption, scalar path, method parameters, and
   exclusions.
3. Add `[IGV]`, `[EAHD]`, and `[HPHD]` to `TODO.md`; expand `[AMAZE]` to cover
   RawTherapee as well as darktable.
4. Run `git diff --check`; expected: exit 0.

## Task 5: Generated artifacts and validation

**Files:** `crates/compare-demosaic/baseline/canonical.json`,
`crates/compare-demosaic/README.md`, snapshots, and the plan checklist.

**Steps:**

1. Run a RawTherapee-only comparison; expected: six shared cases across four
   patterns and no Dual rows.
2. Refresh the full baseline and generated README; review the diff and do not
   waive failing rows.
3. Run the complete validation commands from the Validation section and record
   every outcome.
4. Inspect the final diff, mark completed checklist items, and commit the whole
   implementation as one semantic change.

## Files expected to change

- `scripts/setup.sh`, `scripts/setup.ps1`
- `crates/compare-demosaic/build.rs`
- `crates/compare-demosaic/cpp/algos/**`
- `crates/compare-demosaic/cpp/shim.{h,cpp}`
- `crates/compare-demosaic/src/**`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `crates/compare-demosaic/AGENTS.md`
- `docs/reference/algorithms/rawtherapee.md`
- `docs/_archive/plans/rawtherapee-demosaic-reference.md`
- `TODO.md`

## Validation

```bash
source scripts/setup.sh
cargo test -p compare-demosaic --all-features
cargo run --release -p compare-demosaic -- run --reference rawtherapee
cargo run --release -p compare-demosaic -- run
cargo run --release -p compare-demosaic -- readme
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- \
  -A clippy::too_many_arguments \
  -D clippy::alloc_instead_of_core \
  -D clippy::allow_attributes_without_reason \
  -D clippy::as_underscore \
  -D clippy::bool_to_int_with_if \
  -D clippy::borrow_as_ptr \
  -D clippy::branches_sharing_code \
  -D clippy::clear_with_drain \
  -D clippy::cloned_instead_of_copied \
  -D clippy::collapsible_else_if \
  -D clippy::collection_is_never_read \
  -D clippy::comparison_chain \
  -D clippy::dbg_macro \
  -D clippy::debug_assert_with_mut_call \
  -D clippy::default_trait_access \
  -D clippy::deref_by_slicing \
  -D clippy::doc_comment_double_space_linebreaks \
  -D clippy::doc_include_without_cfg \
  -D clippy::duration_suboptimal_units \
  -D clippy::elidable_lifetime_names \
  -D clippy::empty_enum_variants_with_brackets \
  -D clippy::empty_enums \
  -D clippy::enum_glob_use \
  -D clippy::equatable_if_let \
  -D clippy::explicit_deref_methods \
  -D clippy::explicit_into_iter_loop \
  -D clippy::explicit_iter_loop \
  -D clippy::filter_map_next \
  -D clippy::flat_map_option \
  -D clippy::if_not_else \
  -D clippy::if_then_some_else_none \
  -D clippy::ignored_unit_patterns \
  -D clippy::impl_trait_in_params \
  -D clippy::implicit_clone \
  -D clippy::imprecise_flops \
  -D clippy::inconsistent_struct_constructor \
  -D clippy::inefficient_to_string \
  -D clippy::inline_trait_bounds \
  -D clippy::invalid_upcast_comparisons \
  -D clippy::ip_constant \
  -D clippy::iter_filter_is_ok \
  -D clippy::iter_filter_is_some \
  -D clippy::iter_with_drain \
  -D clippy::large_types_passed_by_value \
  -D clippy::lossy_float_literal \
  -D clippy::manual_assert \
  -D clippy::manual_assert_eq \
  -D clippy::manual_ilog2 \
  -D clippy::manual_instant_elapsed \
  -D clippy::manual_is_power_of_two \
  -D clippy::manual_is_variant_and \
  -D clippy::manual_let_else \
  -D clippy::manual_midpoint \
  -D clippy::manual_string_new \
  -D clippy::map_unwrap_or \
  -D clippy::match_bool \
  -D clippy::match_same_arms \
  -D clippy::match_wild_err_arm \
  -D clippy::missing_const_for_fn \
  -D clippy::mixed_read_write_in_expression \
  -D clippy::mut_mut \
  -D clippy::mutex_integer \
  -D clippy::needless_bitwise_bool \
  -D clippy::needless_collect \
  -D clippy::needless_for_each \
  -D clippy::needless_raw_string_hashes \
  -D clippy::needless_raw_strings \
  -D clippy::needless_type_cast \
  -D clippy::negative_feature_names \
  -D clippy::non_std_lazy_statics \
  -D clippy::nonminimal_bool \
  -D clippy::nonstandard_macro_braces \
  -D clippy::option_as_ref_cloned \
  -D clippy::option_if_let_else \
  -D clippy::option_option \
  -D clippy::or_fun_call \
  -D clippy::pathbuf_init_then_push \
  -D clippy::ptr_offset_by_literal \
  -D clippy::pub_underscore_fields \
  -D clippy::range_minus_one \
  -D clippy::range_plus_one \
  -D clippy::rc_mutex \
  -D clippy::redundant_clone \
  -D clippy::redundant_closure_for_method_calls \
  -D clippy::redundant_else \
  -D clippy::redundant_feature_names \
  -D clippy::redundant_pub_crate \
  -D clippy::ref_binding_to_reference \
  -D clippy::ref_option \
  -D clippy::ref_option_ref \
  -D clippy::rest_pat_in_fully_bound_structs \
  -D clippy::return_and_then \
  -D clippy::same_length_and_capacity \
  -D clippy::search_is_some \
  -D clippy::semicolon_if_nothing_returned \
  -D clippy::semicolon_outside_block \
  -D clippy::set_contains_or_insert \
  -D clippy::should_panic_without_expect \
  -D clippy::single_char_pattern \
  -D clippy::single_match_else \
  -D clippy::single_option_map \
  -D clippy::stable_sort_primitive \
  -D clippy::str_split_at_newline \
  -D clippy::string_lit_as_bytes \
  -D clippy::suboptimal_flops \
  -D clippy::trailing_empty_array \
  -D clippy::trait_duplication_in_bounds \
  -D clippy::try_err \
  -D clippy::uninlined_format_args \
  -D clippy::unnecessary_box_returns \
  -D clippy::unnecessary_debug_formatting \
  -D clippy::unnecessary_join \
  -D clippy::unnecessary_literal_bound \
  -D clippy::unnecessary_self_imports \
  -D clippy::unnecessary_semicolon \
  -D clippy::unnecessary_struct_initialization \
  -D clippy::unnecessary_trailing_comma \
  -D clippy::unnested_or_patterns \
  -D clippy::unseparated_literal_suffix \
  -D clippy::unused_peekable \
  -D clippy::unused_rounding \
  -D clippy::use_self \
  -D clippy::useless_let_if_seq \
  -D clippy::verbose_bit_mask \
  -D clippy::wildcard_dependencies \
  -D clippy::wildcard_imports \
  -D clippy::zero_sized_map_values \
  -D explicit_outlives_requirements \
  -D future_incompatible \
  -D rust_2018_idioms \
  -D rust-2024-compatibility \
  -D unsafe_op_in_unsafe_fn \
  -D unused_extern_crates \
  -D unused_import_braces \
  -D unused_lifetimes \
  -D warnings
git diff --check
```

## Decisions and notes

- RawTherapee 5.13 is the pinned release.
- The integration follows the existing LibRaw/darktable source-adapter pattern:
  materialize only required upstream files and compile them behind small
  compatibility headers.
- Scalar RawTherapee source branches are used to avoid a full GTK/GLib/FFTW
  dependency and keep the reference deterministic across supported platforms.
- The comparison supplies an identity camera matrix and no white balance.
- DCB uses `(0, true)` and LMMSE uses one iteration so the compared variants
  correspond to the existing LibRaw/darktable reference semantics.
- IGV, EAHD, and HPHD remain adapter-only until matching Fornax implementations
  exist.
- The compatibility output planes include two guard rows and columns because
  RawTherapee 5.13's AMaZE scalar path can write one pixel past odd-width GRBG
  output. Only the requested image plane is copied back.
- RawTherapee's LMMSE and color LUT facade clamps floating-point indices to the
  table bounds, matching the upstream LUT clipping convention.
- Workspace clippy currently reports pre-existing lints in untouched
  `fornax-raw-pipeline` files. The same strict lint invocation on
  `compare-demosaic` reaches two pre-existing redundant-clone findings in
  `diff.rs` and `metrics.rs`; no new RawTherapee finding is present.
