# AMaZE faithful-port implementation plan

## Goal

Replace Fornax's generic directional approximation with a dedicated AMaZE
implementation that reproduces darktable 5.6.2 and RawTherapee 5.13 closely
enough for both native `GBRG` comparison rows to pass their existing robust
limits, then retire the obsolete `Amaze` shared-profile branch and record the
accepted baseline.

## Scope

In scope:

- the AMaZE tile pipeline from normalized Bayer CFA through RGB output;
- adaptive green ratios, directional variance weighting, Nyquist texture
  detection and refinement, diagonal red/blue correction, chroma
  interpolation, and final clipping/quantization;
- tests for tile geometry, directional decisions, measured-site preservation,
  uniform fields, all four Bayer phases, and lazy row access;
- the darktable and RawTherapee comparison rows, snapshots, generated README,
  canonical baseline, reference note, and `[AMAZE]` completion record.

Out of scope:

- camera white balance, camera matrices, decoder or preprocessor changes;
- custom tile sizes or parallel scheduling changes;
- X-Trans, four-color, monochrome, or non-2x2 Bayer layouts;
- changing comparison limits or waiving a failing row.

## Task checklist

- [x] Start from current `origin/main` in `.worktrees/amaze-implement`.
- [x] Watch the existing native `GBRG` darktable and RawTherapee comparisons
  fail before implementation.
- [x] Add failing stage tests for tile coverage and 16-pixel reflection,
  directional ratio selection, local variance selection, Nyquist refinement,
  and diagonal red/blue selection.
- [x] Implement dedicated AMaZE tiling and green/color-difference
  reconstruction.
- [x] Implement Nyquist area interpolation/refinement, diagonal red/blue
  reconstruction, and final chroma interpolation.
- [x] Preserve measured CFA samples and support all four conventional Bayer
  phases.
- [x] Remove `Amaze` from `directional.rs`, retaining the module for the
  separate DHT migration.
- [x] Iterate the two native `GBRG` comparison rows with partial runs and
  inspect the full-frame texture error map.
- [x] Update AMaZE snapshots, notes, baseline, generated README, and `TODO.md`.
- [x] Run crate-specific and workspace validation from the repository root.
- [x] Inspect the final diff, commit, push, and open the task pull request.

## Files expected to change

- `crates/fornax-postprocessor/src/demosaic/amaze.rs`
- `crates/fornax-postprocessor/src/demosaic.rs`
- `crates/fornax-postprocessor/src/demosaic/directional.rs` (delete)
- `crates/compare-demosaic/src/snapshots/*-AMaZE-{darktable,rawtherapee}.snap`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `docs/references/amaze.md`
- `docs/plans/amaze-implement.md`
- `TODO.md`

## Validation

Iterate with:

```bash
cargo test -p fornax-postprocessor
cargo run --release -p compare-demosaic -- run \
  --algorithm amaze --pattern GBRG --artifacts --out target/amaze
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/amaze/canonical.json
```

Accept only when both native `GBRG` AMaZE rows have `MAE <= 2`, `P99 <= 64`,
and high-error fraction at most `0.001` above 1024 codes. Then:

```bash
cargo insta test -p compare-demosaic
cargo insta accept
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json \
  crates/compare-demosaic/baseline/canonical.json
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

- Use `AMAZETS = 160`, a 16-pixel halo, tile starts at `-16`, and a 128-pixel
  tile stride, matching the pinned references.
- Keep the Rust implementation in `f32` over normalized samples and perform
  the harness-compatible nearest-code conversion once at the lazy boundary.
- The user explicitly requested implementation on branch
  `task/amaze-implement`; darktable/RawTherapee remain behavioral evidence,
  and direct source reuse is not part of this independent Rust port. The
  licensing caveat remains recorded in the reference note.
- RawTherapee's comparison facade uses `initialGain = 1` and no white balance;
  the port must match the common uncorrected mosaic convention rather than
  introducing pipeline-only channel scaling.
- The RawTherapee compatibility `xdivf(value, exponent)` must preserve the
  upstream exponent-shift convention; the previous divisor interpretation
  inflated AMaZE directional weights from about `0.5` to about `50`.
- A failing row is never waived. If one reference cannot meet the robust limits
  after using the common convention, stop and report the adapter/reference
  conflict rather than relaxing limits.
- Validation results:
  - Native GBRG against both references: MAE `0.230396`, P99 `1`, maximum
    `890`, and zero samples above 1024 codes.
  - `cargo test --workspace --all-features`: 260 passed.
  - `cargo +nightly fmt --check`: passed.
  - `cargo clippy -p fornax-postprocessor --all-features --all-targets` with
    `-D warnings` and `too_many_arguments` allowed: passed.
  - `cargo clippy -p compare-demosaic --all-features --all-targets` with
    `-D warnings` and `too_many_arguments` allowed: passed.
  - The repository-wide strict clippy command still fails in untouched
    pre-existing code in `fornax-preprocessor`, `fornax-decoder-rawler`,
    `bench-decoder`, `linear.rs`, and `lmmse.rs`.
  - `cargo insta test -p compare-demosaic`: 149 passed, no snapshots to review.
  - Full comparison run and baseline diff: no differences.
  - `readme --check`: up to date.
  - Markdown lint was not run because `markdownlint-cli2` is not installed in
    the available Pixi environments or PATH.
