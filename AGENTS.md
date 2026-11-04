# Agent instructions — fornax

## Overview

Fornax is a Rust workspace for decoding RAW images, normalizing Bayer mosaics,
and reconstructing RGB output through separate pipeline stages. When
instructions conflict, the user's explicit request wins. Otherwise, the
`AGENTS.md` closest to the files being changed wins over this root file.

## Project map

Read [docs/architecture.md](docs/architecture.md) for crate roles, interfaces,
and pipeline data flow. These invariants always apply:

- Keep decoding, preprocessing, and demosaicing as separate stages.
- The active demosaic target is conventional 2x2 RGB Bayer input. Do not claim
  support for X-Trans, four-color, monochrome, or other CFA layouts.

## Choosing work

Read `TODO.md`, the human-owned backlog and completion record, before selecting
a bracketed task ID. Branch every request from the current `origin/main`. One
request maps to one branch, one worktree, and one pull request.

| Request | Branch | Worktree | Pull request requirements |
| --- | --- | --- | --- |
| `TODO.md` item selected by bracketed ID | `task/<lowercase-id>-<slug>` | `.worktrees/<lowercase-id>-<slug>` | Include the task ID and remove the item from `TODO.md` in the same pull request. Keep the item if work is incomplete or blocked. |
| GitHub issue number supplied by the user | `fix/<number>-<slug>` | `.worktrees/fix-<number>-<slug>` | Read only that issue with `gh issue view <number>`, add regression coverage, and include `Fixes #<number>` in the body. |
| Explicit request not in `TODO.md` | `chore/<slug>` | `.worktrees/chore-<slug>` | Leave `TODO.md` untouched unless the request names an item. |

Use a short lowercase kebab-case slug.
Modify `TODO.md` only for a completed selected task, or when the request names an item.

## Workflow

Fetch and create the worktree from `origin/main`:

```bash
git fetch origin
git worktree add -b "<branch>" ".worktrees/<worktree>" origin/main
```

Implement only the requested change and its required tests or documentation.
Run validation, then inspect the diff before committing:

```bash
git add <paths>
git commit -m "<type>: <summary>"
git push -u origin HEAD
gh pr create --base main --title "<type>: <summary>" --body "<body>"
```

Open one semantic pull request and report its URL. Put the task ID, issue
reference, or maintenance summary in the body as applicable.

## Plans

Write a plan only for non-trivial work: several files or crates, a design
decision, or more than a short session. Small items go straight to
implementation.

Store plans under `docs/plans/` using the branch ID and slug:

- Task: `docs/plans/<lowercase-id>-<slug>.md`
- Issue: `docs/plans/fix-<number>-<slug>.md`
- Chore: `docs/plans/chore-<slug>.md`

Commit the plan in the same pull request. Update its progress and record
decisions while the work proceeds; stop treating it as authoritative once the
pull request merges.

Every plan must contain Goal, Scope (in and out), Task checklist, Files
expected to change, Validation, and Decisions and notes. Derive the detailed
checklist from the selected `TODO.md` item and the code without expanding
`TODO.md`. Make the plan self-contained: a reader with only the working tree
and that file must be able to carry out the work.

## Validation

For Rust changes, run from the repository root:

```bash
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets --
      -D clippy::alloc_instead_of_core
      -D clippy::allow_attributes_without_reason
      -D clippy::as_underscore
      -D clippy::bool_to_int_with_if
      -D clippy::borrow_as_ptr
      -D clippy::branches_sharing_code
      -D clippy::clear_with_drain
      -D clippy::cloned_instead_of_copied
      -D clippy::collapsible_else_if
      -D clippy::collection_is_never_read
      -D clippy::comparison_chain
      -D clippy::dbg_macro
      -D clippy::debug_assert_with_mut_call
      -D clippy::default_trait_access
      -D clippy::deref_by_slicing
      -D clippy::doc_comment_double_space_linebreaks
      -D clippy::doc_include_without_cfg
      -D clippy::duration_suboptimal_units
      -D clippy::elidable_lifetime_names
      -D clippy::empty_enum_variants_with_brackets
      -D clippy::empty_enums
      -D clippy::enum_glob_use
      -D clippy::equatable_if_let
      -D clippy::explicit_deref_methods
      -D clippy::explicit_into_iter_loop
      -D clippy::explicit_iter_loop
      -D clippy::filter_map_next
      -D clippy::flat_map_option
      -D clippy::if_not_else
      -D clippy::if_then_some_else_none
      -D clippy::ignored_unit_patterns
      -D clippy::impl_trait_in_params
      -D clippy::implicit_clone
      -D clippy::imprecise_flops
      -D clippy::inconsistent_struct_constructor
      -D clippy::inefficient_to_string
      -D clippy::inline_trait_bounds
      -D clippy::invalid_upcast_comparisons
      -D clippy::ip_constant
      -D clippy::iter_filter_is_ok
      -D clippy::iter_filter_is_some
      -D clippy::iter_with_drain
      -D clippy::large_types_passed_by_value
      -D clippy::lossy_float_literal
      -D clippy::manual_assert
      -D clippy::manual_assert_eq
      -D clippy::manual_ilog2
      -D clippy::manual_instant_elapsed
      -D clippy::manual_is_power_of_two
      -D clippy::manual_is_variant_and
      -D clippy::manual_let_else
      -D clippy::manual_midpoint
      -D clippy::manual_string_new
      -D clippy::map_unwrap_or
      -D clippy::match_bool
      -D clippy::match_same_arms
      -D clippy::match_wild_err_arm
      -D clippy::missing_const_for_fn
      -D clippy::mixed_read_write_in_expression
      -D clippy::mut_mut
      -D clippy::mutex_integer
      -D clippy::needless_bitwise_bool
      -D clippy::needless_collect
      -D clippy::needless_for_each
      -D clippy::needless_raw_string_hashes
      -D clippy::needless_raw_strings
      -D clippy::needless_type_cast
      -D clippy::negative_feature_names
      -D clippy::non_std_lazy_statics
      -D clippy::nonminimal_bool
      -D clippy::nonstandard_macro_braces
      -D clippy::option_as_ref_cloned
      -D clippy::option_if_let_else
      -D clippy::option_option
      -D clippy::or_fun_call
      -D clippy::pathbuf_init_then_push
      -D clippy::ptr_offset_by_literal
      -D clippy::pub_underscore_fields
      -D clippy::range_minus_one
      -D clippy::range_plus_one
      -D clippy::rc_mutex
      -D clippy::redundant_clone
      -D clippy::redundant_closure_for_method_calls
      -D clippy::redundant_else
      -D clippy::redundant_feature_names
      -D clippy::redundant_pub_crate
      -D clippy::ref_binding_to_reference
      -D clippy::ref_option
      -D clippy::ref_option_ref
      -D clippy::rest_pat_in_fully_bound_structs
      -D clippy::return_and_then
      -D clippy::same_length_and_capacity
      -D clippy::search_is_some
      -D clippy::semicolon_if_nothing_returned
      -D clippy::semicolon_outside_block
      -D clippy::set_contains_or_insert
      -D clippy::should_panic_without_expect
      -D clippy::single_char_pattern
      -D clippy::single_match_else
      -D clippy::single_option_map
      -D clippy::stable_sort_primitive
      -D clippy::str_split_at_newline
      -D clippy::string_lit_as_bytes
      -D clippy::suboptimal_flops
      -D clippy::trailing_empty_array
      -D clippy::trait_duplication_in_bounds
      -D clippy::try_err
      -D clippy::uninlined_format_args
      -D clippy::unnecessary_box_returns
      -D clippy::unnecessary_debug_formatting
      -D clippy::unnecessary_join
      -D clippy::unnecessary_literal_bound
      -D clippy::unnecessary_self_imports
      -D clippy::unnecessary_semicolon
      -D clippy::unnecessary_struct_initialization
      -D clippy::unnecessary_trailing_comma
      -D clippy::unnested_or_patterns
      -D clippy::unseparated_literal_suffix
      -D clippy::unused_peekable
      -D clippy::unused_rounding
      -D clippy::use_self
      -D clippy::useless_let_if_seq
      -D clippy::verbose_bit_mask
      -D clippy::wildcard_dependencies
      -D clippy::wildcard_imports
      -D clippy::zero_sized_map_values
      -D explicit_outlives_requirements
      -D future_incompatible
      -D rust_2018_idioms
      -D rust-2024-compatibility
      -D unsafe_op_in_unsafe_fn
      -D unused_extern_crates
      -D unused_import_braces
      -D unused_lifetimes
      -D warnings
```

Also run task-specific tests, benchmarks, or examples. For documentation-only
changes, run `git diff --check` and the repository-configured Markdown lint on
the changed Markdown. Record commands and outcomes in the pull request. Report
failures and skipped checks explicitly; never describe an unrun check as
passing.

## Pull request handoff

Titles must satisfy `.github/semantic.yml`; for example, use
`docs: rewrite agent guidance`. Put requested changes on the same branch and
pull request. Do not watch, poll, merge, or manually close a pull request or
linked issue. Only the user merges or abandons a pull request.

## Boundaries

- Do not commit binary files to the book; CI enforces this through
  `book/scripts/check_assets.py`.
- Project skills live in `.agents/skills/`. Add or improve them by pull
  request. Skills installed by pixi are managed by pixi; never edit them by
  hand.
- Do not add items to `TODO.md`; propose them in the report or pull request
  description.
- Keep the repository root clean: the only agent-related root files are
  `AGENTS.md` and `TODO.md`. Do not add tool-specific instruction files for a
  particular coding agent.

## Reference knowledge

- Preserve reusable algorithm and implementation research in
  `docs/references/<topic>.md` and commit it with the code it informs. Do not
  leave important findings only in chat, scratch files, or pull request
  comments.
- Record authoritative source links with pinned versions or commits, the
  behavior or decision they support, known implementation differences, and
  license constraints that affect reuse.
- Link implementation modules or generated documentation to the relevant
  reference note when the source material clarifies invariants that are easy
  to regress.

## Where to look next

- Architecture and pipeline boundaries: [docs/architecture.md](docs/architecture.md)
- Per-work plans: [docs/plans/](docs/plans/)
- Algorithm and reference notes: [docs/references/](docs/references/)
- Comparison-crate rules:
  [crates/compare-demosaic/AGENTS.md](crates/compare-demosaic/AGENTS.md)
- Human-facing book: [book/](book/README.md); agents edit it only when asked
  and follow [book/AGENTS.md](book/AGENTS.md).
- Project procedures: [.agents/skills/](.agents/skills/)
- Use XERJ only to locate unfamiliar external reference code; verify hits at
  `file:line`. Never substitute it for reading `TODO.md`, `AGENTS.md`, generated
  README tables, Cargo output, or diffs.
