# Demosaic linear workflow and parameter audit

## Goal

Make every demosaic's linear input/output boundary explicit, expose the missing
DCB and LMMSE behavioral controls directly on `Demosaic`, and fix the
postprocessor's red/blue white-balance mutation while preserving current default
outputs and comparison baselines.

## Scope

**In scope:**

- A linear-boundary audit and regression coverage for all eleven demosaics.
- Struct variants `Demosaic::Dcb { iterations, enhance }` and
  `Demosaic::Lmmse { mode }`.
- A public `LmmseMode` covering darktable's basic, median, three-median, and
  refinement modes.
- Default-preserving constructors `Demosaic::dcb()` and `Demosaic::lmmse()`.
- DCB iteration/enhancement sequencing, LMMSE median/refinement sequencing,
  DCB/LMMSE reference-adapter test hooks, public API documentation, and
  corrected white balance.
- Migration of every in-repository call site, example, benchmark, test, and
  comparison runner to the new enum shape.

**Out of scope:**

- Camera white balance, camera matrices, decoder, or preprocessor changes.
- X-Trans, four-color, monochrome, or other CFA layouts.
- Exposing tile sizes, scheduling controls, AMaZE initial gain, or LMMSE
  scaler.
- New comparison rows, baseline refreshes, limit changes, or waivers.

## Task checklist

- [ ] Add failing tests for linear boundary preservation, default
  `Demosaic::dcb()`/`Demosaic::lmmse()` behavior, configurable DCB sequencing,
  configurable LMMSE sequencing, and independent red/blue white balance.
- [ ] Change `Demosaic` to carry DCB iterations/enhancement and LMMSE mode
  inline; update all internal callers and public examples.
- [ ] Implement parameter-aware DCB and LMMSE paths with default behavior
  identical to the current comparison output.
- [ ] Extend comparison reference adapters with test-only parameter hooks and
  verify non-default modes against their pinned references.
- [ ] Update the postprocessor and DCB/LMMSE reference documentation.
- [ ] Run focused tests, comparison checks, workspace validation, and inspect
  the final diff.

## Tasks

### Task 1: Public enum and linear boundary

**Files:**

- Modify: `crates/fornax-postprocessor/src/demosaic.rs`
- Modify: `crates/fornax-postprocessor/src/lib.rs`
- Modify: `crates/fornax-postprocessor/benches/support/demosaic_registry.rs`

**Interfaces:**

- Produces: `Demosaic::Dcb { iterations: u8, enhance: bool }`
- Produces: `Demosaic::Lmmse { mode: LmmseMode }`
- Produces: `LmmseMode::{Basic, Median, ThreeMedians, Refine(NonZeroU8)}`
- Produces: default-preserving `Demosaic::dcb()` and `Demosaic::lmmse()`

- [ ] Write a failing all-algorithm linear-boundary test using low-level
  uniform mosaics in both `f32` and `f64`.
- [ ] Run `cargo test -p fornax-postprocessor all_algorithms_preserve_linear_boundary`;
  expect failure because the struct variants do not exist.
- [ ] Change `Demosaic` and update dispatch, defaults, constructors, and the
  benchmark registry.
- [ ] Run the focused test and then `cargo test -p fornax-postprocessor`;
  expect all tests to pass.
- [ ] Commit the task.

### Task 2: Configurable DCB

**Files:**

- Modify: `crates/fornax-postprocessor/src/demosaic/dcb.rs`

**Interfaces:**

- Consumes: `Demosaic::Dcb { iterations, enhance }`
- Produces: `DemosaicDcb::lazy_with_options(..., iterations: u8, enhance: bool)`

- [ ] Add failing tests proving multiple Nyquist rounds run before color
  reconstruction and that disabling enhancement skips refinement/full color.
- [ ] Run the focused tests; expect failure because the options are not wired.
- [ ] Thread the options through `run` in the same order as LibRaw's `dcb()`
  while preserving the `(0, true)` default path.
- [ ] Run the focused tests and then `cargo test -p fornax-postprocessor`;
  expect all tests to pass.
- [ ] Commit the task.

### Task 3: Configurable LMMSE

**Files:**

- Modify: `crates/fornax-postprocessor/src/demosaic/lmmse.rs`

**Interfaces:**

- Consumes: `Demosaic::Lmmse { mode }` and `LmmseMode`
- Produces: `DemosaicLmmse::lazy_with_mode(...)`

- [ ] Add failing tests proving `Basic`, `ThreeMedians`, and `Refine(1)` select
  the darktable median/refinement counts while `Median` preserves current output.
- [ ] Run the focused tests; expect failure because mode dispatch is absent.
- [ ] Implement median loops and the pinned darktable refinement pass.
- [ ] Run the focused tests and then `cargo test -p fornax-postprocessor`;
  expect all tests to pass.
- [ ] Commit the task.

### Task 4: White balance boundary fix

**Files:**

- Modify: `crates/fornax-postprocessor/src/lib.rs`

**Interfaces:**

- Produces: independent red and blue channel multipliers in `PostProcessor`.

- [ ] Add a failing white-balance test with unequal red and blue multipliers.
- [ ] Run it; expect the blue sample to be incorrectly tied to red.
- [ ] Multiply each color by its own pre-multiplied sample.
- [ ] Run the focused test and then `cargo test -p fornax-postprocessor`;
  expect all tests to pass.
- [ ] Commit the task.

### Task 5: Reference checks and documentation

**Files:**

- Modify: `crates/compare-demosaic/src/reference.rs`
- Modify: comparison C/C++ adapters under `crates/compare-demosaic/cpp/`
- Modify: `docs/references/dcb.md`
- Modify: `docs/references/lmmse.md`
- Modify: `crates/fornax-postprocessor/README.md`

**Interfaces:**

- Consumes: parameterized DCB and LMMSE implementations from Tasks 2 and 3.
- Produces: test-only reference calls for non-default DCB/LMMSE settings.

- [ ] Add failing synthetic comparisons for non-default DCB and LMMSE settings
  against their pinned references.
- [ ] Run the focused comparison tests; expect failure because reference option
  dispatch is absent.
- [ ] Extend the C bridge and adapters with explicit options while keeping all
  table-driven default cases unchanged.
- [ ] Document the public API, linear boundary, mode mapping, and fixed
  comparison defaults.
- [ ] Run the full validation section and inspect the final diff.
- [ ] Commit the task.

## Files expected to change

- `crates/fornax-postprocessor/src/demosaic.rs`
- `crates/fornax-postprocessor/src/demosaic/dcb.rs`
- `crates/fornax-postprocessor/src/demosaic/lmmse.rs`
- `crates/fornax-postprocessor/src/lib.rs`
- `crates/fornax-postprocessor/README.md`
- `crates/fornax-postprocessor/benches/support/demosaic_registry.rs`
- `crates/compare-demosaic/src/cases.rs`
- `crates/compare-demosaic/src/reference.rs`
- `crates/compare-demosaic/cpp/shim.h`
- `crates/compare-demosaic/cpp/shim.cpp`
- `crates/compare-demosaic/cpp/algos/libraw_dcb.cpp`
- `crates/compare-demosaic/cpp/algos/rawtherapee_dcb.cpp`
- `crates/compare-demosaic/cpp/algos/darktable_lmmse.c`
- `crates/compare-demosaic/cpp/algos/rawtherapee_lmmse.cpp`
- `docs/references/dcb.md`
- `docs/references/lmmse.md`
- `docs/plans/chore-demosaic-parameter-audit.md`

## Validation

```bash
cargo test -p fornax-postprocessor
cargo test -p compare-demosaic
cargo run --release -p compare-demosaic -- run --algorithm dcb --out target/demosaic-params
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/demosaic-params/canonical.json
cargo run --release -p compare-demosaic -- run --algorithm lmmse --out target/demosaic-params
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/demosaic-params/canonical.json
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
pixi exec -s markdownlint-cli2 markdownlint-cli2 \
  crates/fornax-postprocessor/README.md \
  docs/references/dcb.md \
  docs/references/lmmse.md \
  docs/plans/chore-demosaic-parameter-audit.md
git -c core.whitespace=cr-at-eol diff --check
```

## Decisions and notes

- The user selected the linear-boundary interpretation: reference-mandated
  internal gamma transforms remain valid as long as normalized linear input
  and output are preserved.
- The user selected behavioral controls rather than every upstream knob.
- The user requested the controls inline on `Demosaic`; no `DemosaicConfig`
  compatibility wrapper is introduced, and the source-breaking enum change is
  intentional.
- Defaults remain DCB `(iterations = 0, enhance = true)` and LMMSE mode 1, so
  existing comparison rows should not move.
