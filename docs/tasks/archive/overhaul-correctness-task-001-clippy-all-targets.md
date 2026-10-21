# Overhaul Correctness Task 001: Clippy For All Targets

Status: Complete - 2026-10-20. `cargo clippy --all-targets -- -D warnings` is Green, and the test counts did not change. The task has no visual check.

## Goal

`cargo clippy --all-targets -- -D warnings` is Green, and the gate in `AGENTS.md` uses it. The test code then has the same lint discipline as the product code.

## Authority

- [AGENTS.md](../../../AGENTS.md) owns the gate: "Every commit passes `cargo check`, `cargo fmt --check`, and `cargo clippy -- -D warnings`". This task changes that line and the "Build, Test, Lint" block.
- The [overhaul plan](../../plans/design-and-cleanup-overhaul-plan.md), "Throughout: Code Correctness": add `cargo clippy --all-targets -- -D warnings` to the gate.
- Operator decision of 2026-10-20: one cleanup packet with no behavior change.

## Recorded Facts - 2026-10-20

- `cargo clippy --all-targets -- -D warnings` gives about 95 errors, each in test code or in a test-only path.
- The most frequent lints: `doc_markdown` (13), `items_after_statements` (12), `default_trait_access` (more than 20 in total), `float_cmp` (7), `unused_io_amount` (5), `single_element_loop` (5), `duration_suboptimal_units` (5), `items_after_test_module` (4), `assertions_on_constants` (3).
- The files with the most errors: `src/library/app_impl.rs` (10), `src/app/bootstrap.rs` (10), `src/ui/composites/maintenance_page.rs` (6), `src/rss/check_apply.rs` (6), `src/view_models/startup/capabilities.rs` (5), `src/runtime/playlist_rss_check.rs` (5), `tests/architecture_tests.rs` (4).
- The repository has no CI workflow file. `AGENTS.md` says that CI runs build, test, clippy and format check.

## Design

- Correct each lint at its place. Prefer the fix that clippy proposes. A test that compares `f32` or `f64` uses an explicit tolerance. It can also use `assert_eq!` on a value that is exact by construction, with a comment that says why.
- `#[expect(clippy::...)]` with a `reason` is allowed only when the fix makes a test less clear. Each `expect` names the lint and the reason. No `allow`.
- Change no product behavior and no test assertion meaning.
- Change the gate in `AGENTS.md`: the working rule "Zero warnings" and the "Build, Test, Lint" block use `cargo clippy --all-targets -- -D warnings`. Correct the CI sentence: the repository has no CI workflow file, so the sentence states that the gate runs locally.

## Required Changes

1. Correct each error of `cargo clippy --all-targets -- -D warnings`.
2. Change the gate lines of `AGENTS.md`.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| RC1-01 | `cargo clippy --all-targets -- -D warnings` is Green |
| RC1-02 | `cargo test` gives the same count of passing tests as before the task |
| RC1-03 | `git diff` shows no change to product code outside a `#[cfg(test)]` path, except a lint fix that the report names |

## Visual Acceptance Criteria

None. This task changes no screen.

## Exclusions

- No new lint group, and no change to `#![warn(clippy::pedantic)]`.
- No refactor of a test beyond its lint.

## Checks

```bash
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo build --bin v4vmm
```

## Rollback

Revert the commit.

## Implementation Result - 2026-10-20

`cargo clippy --all-targets -- -D warnings` fixed 99 errors in 47 files. `cargo test` gave
2056 passed library tests and 342 passed `architecture_tests`, before the task and after
it. The counts did not change.

### Lints Fixed By Kind

| Lint | Count |
|---|---|
| `doc_markdown` | 13 |
| `items_after_statements` | 12 |
| `default_trait_access` | 23 |
| `float_cmp` | 7 |
| `single_element_loop` | 5 |
| `unused_io_amount` | 5 |
| `duration_suboptimal_units` | 5 |
| `items_after_test_module` | 4 |
| `assertions_on_constants` | 3 |
| `too_many_lines` | 4 |
| `needless_pass_by_value` | 2 |
| `cloned_ref_to_slice_refs` | 2 |
| `unchecked_time_subtraction` | 2 |
| `cast_possible_truncation` | 2 |
| `elidable_lifetime_names` | 2 |
| `needless_raw_string_hashes` | 1 |
| `unnecessary_wraps` (`publisher_targets.rs`) | 1 |
| `unnecessary_wraps` (`registry.rs`) | 1 |
| `unnecessary_wraps` (`detail_selection.rs`) | 1 |
| `non_octal_unix_permissions` | 1 |
| `type_complexity` | 1 |
| `format_collect` | 1 |
| `struct_field_names` | 1 |

### Lints Fixed By File

| File | Count |
|---|---|
| `src/app/bootstrap.rs` | 10 |
| `src/library/app_impl.rs` | 10 |
| `src/ui/composites/maintenance_page.rs` | 6 |
| `src/runtime/playlist_rss_check.rs` | 5 |
| `src/view_models/startup/capabilities.rs` | 5 |
| `tests/architecture_tests.rs` | 4 |
| `src/rss/check_apply.rs` | 4 |
| `src/view_models/log_view.rs` | 4 |
| `src/app/resize.rs` | 3 |
| `src/view_models/workspace/tests.rs` | 3 |
| `src/broadcast/publisher_targets.rs` | 2 |
| `src/app.rs` | 2 |
| `src/db.rs` | 2 |
| `src/provider_observation/http.rs` | 2 |
| `src/view_models/startup.rs` | 2 |
| `src/view_models/library.rs` | 2 |
| `src/view_models/publisher_page.rs` | 2 |
| `src/view_models/track_detail.rs` | 2 |

Each of these files had one error:

- `src/app/keyboard.rs`
- `src/app/search_dispatch.rs`
- `src/application/commands/maintenance.rs`
- `src/application/queries/stored_values.rs`
- `src/broadcast/registry.rs`
- `src/config.rs`
- `src/db/maintenance/preservation.rs`
- `src/db/payment_routes.rs`
- `src/db/provider_observations.rs`
- `src/db/rss_check_runs.rs`
- `src/db/rss_field_holds.rs`
- `src/rss/subscribe.rs`
- `src/runtime/broadcast_readiness.rs`
- `src/runtime/musicbrainz_feed_saga.rs`
- `src/startup/fixture.rs`
- `src/track_compare.rs`
- `src/ui/composites/show_log_pane.rs`
- `src/ui/composites/thumbnail.rs`
- `src/ui/composites/track_metadata_grid.rs`
- `src/ui/composites/settings.rs`
- `src/ui/layouts.rs`
- `src/ui/primitives/button.rs`
- `src/ui/shells/workspace.rs`
- `src/ui/tokens.rs`
- `src/view_models/startup/database.rs`
- `src/view_models/detail_selection.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/show.rs`

### Each `#[expect]`, With Its Reason

All nine `#[expect]` attributes name `reason` and no change uses `#[allow]`.

| Lint | Location | `reason` |
|---|---|---|
| `clippy::too_many_lines` | `src/app/bootstrap.rs`, `adr_0066_drained_core_reopens_fresh_only_after_successful_checks` | This test narrates one sequential drain, correction and resume path. Splitting it can scatter shared session and path data. |
| `clippy::too_many_lines` | `src/app/keyboard.rs`, `adr_0071_button_enter_precedes_active_pane_shortcut` | This test defines a local view and simulates key events against one shared window. Splitting it can scatter that shared data. |
| `clippy::too_many_lines` | `src/application/commands/maintenance.rs`, `adr_0066_restore_command_requires_review_drain_and_unchanged_configuration` | This test loops over five restore scenarios that share one session, connection and config. Splitting can scatter that data. |
| `clippy::unnecessary_wraps` | `src/broadcast/registry.rs`, `fixed_time` | `fixed_time` must agree with the production `Clock` type, `fn() -> Result<i64>`. |
| `clippy::unused_io_amount` (5 sites) | `src/db/provider_observations.rs`, `src/provider_observation/http.rs` (two sites), `src/rss/subscribe.rs`, `src/track_compare.rs` | The mock server drains one read into an oversized buffer, so `read_exact` can wait with no end. |

The fourth `too_many_lines` error, in `src/view_models/library.rs`, uses a different fix.
It does not use `#[expect]`. Two helper functions, `sample_observation_write_failure`
and `sample_observation_receipt`, hold the struct literals. The test calls them.

### Product-Code Changes

`git diff` shows four files with a change in product code, not in a `#[cfg(test)]` path.
Each one moves an existing item, with no change to its body, to fix `items_after_test_module`:

- `src/ui/tokens.rs`: `LOG_TEXT_SIZE`, `LOG_LINE_HEIGHT` and `log_font_family` move from
  after `mod tests` to before it.
- `src/library/app_impl.rs`: `playlist_options` moves from after `mod tests` to before it.
- `src/startup/fixture.rs`: `observe_session`, `record_session_release`,
  `append_session_observation` and `seed_upgrade_checks` move from after `mod tests` to
  before it.
- `src/ui/composites/track_metadata_grid.rs`: `metadata_heading_cell` moves from after
  `mod tests` to before it.

A fifth change touches `src/library/app_impl.rs`. The moved test helper spells
`FeedStoredValues::default()` with its full crate path. The scope near its new position
has no import for the bare type name.

### Checks

`cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --test architecture_tests`,
`cargo fmt -- --check` and `cargo build --bin v4vmm` are each Green.

### Visual Check

None. This task changed no screen.

## Prompt for lower-context coding model

You are implementing one bounded task.

Read:
- `AGENTS.md`
- This packet: `docs/tasks/overhaul-correctness-task-001-clippy-all-targets.md`

Goal:
- Make `cargo clippy --all-targets -- -D warnings` Green with no behavior change, and change the gate lines of `AGENTS.md`.

Constraints:
- Follow the rust-dev skill. Write each comment in ASD-STE100 Simplified Technical English (`~/.agents/skills/asd-ste100/SKILL.md`).
- Record the test count before the first edit, and compare it at the end.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`. Do not run the app.

Do not touch:
- Any ADR, and each document other than this packet and the gate lines of `AGENTS.md`.

At the end, report the lints fixed by kind and by file, and each `#[expect]` with its reason.
Also report each product-code change, and the test counts before and after.

Stop and report the problem, and do not guess, when:
- A lint fix changes the meaning of a test or of product code.
