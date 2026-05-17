# ADR 0050 Task 001 — decompose `src/app.rs`

## Goal

Move three handler clusters out of `src/app.rs` into new `src/app/`
submodules without changing behavior:

- `src/app/search_dispatch.rs` — toolbar search submit, result-row drill-down,
  Index search async wiring, search-results detail sync, remote thumbnail
  state.
- `src/app/breadcrumb.rs` — ContentList breadcrumb segment handling.
- `src/app/resize.rs` — content-pane fluid resize handlers.

After this task, `src/app.rs` shrinks to ~1,800-2,000 LOC and holds only
`TopApp` struct + `Render` impl + `render_workspace_content` dispatcher +
`Application` boot + module wiring.

## Files To Inspect

- `src/app.rs`
- `src/app/mod.rs` (if present) and existing submodules:
  `bootstrap.rs`, `events.rs`, `keyboard.rs`, `menu.rs`, `playback_bar.rs`,
  `queue_now_playing.rs`, `tab_bar.rs`
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `tests/architecture_tests.rs` (for guards that pin `src/app.rs`)

## Files Likely To Change

- `src/app.rs` (shrinks)
- `src/app/search_dispatch.rs` (new)
- `src/app/breadcrumb.rs` (new)
- `src/app/resize.rs` (new)
- `tests/architecture_tests.rs` (path retargeting only, no logic change)

## Do Not Touch

- Render output of any UI.
- Public API of `TopApp` or any of its methods.
- View-model code (`src/view_models/*`).
- UI composite or shell code.
- The `discover/` module.
- Any text-filter helper logic.
- Commit Stage gates: never skip hooks.

## Constraints

- Behavior-preserving move only. No new methods, no signature changes, no
  inlined helpers being de-inlined.
- Use `git mv` semantics: each submodule extraction lands as its own
  commit so blame survives via `git log --follow`.
- Keep `impl TopApp` blocks in submodules; do not introduce a trait or new
  type to wrap the methods.
- Re-export discipline: nothing in `src/app.rs` should `use` the new
  submodule unless the method body inside the submodule needs to call back
  into the parent. The submodule declarations (`mod search_dispatch;` etc.)
  are in `src/app.rs`.
- No commit unless explicitly asked (the operator runs commits themselves).
  Stage the changes; report what is ready to commit.

## Implementation Steps

1. Read `src/app.rs` in full. Identify the exact line ranges that own:
   - `submit_global_search`, `submit_global_search_with`,
     `handle_search_result_selected`, `start_index_search_for_query`,
     `sync_search_results_detail_with_nav`, and `RemoteDetailThumbnailState`
     (search_dispatch cluster).
   - `handle_content_list_breadcrumb_select` plus any private breadcrumb
     labeler helpers it calls (breadcrumb cluster).
   - `begin_content_pane_resize`, `resize_content_pane`,
     `end_content_pane_resize`, `is_content_pane_resizing`, and any
     `content_pane_width` accessors that aren't trivially auto-generated
     (resize cluster).
2. Create `src/app/search_dispatch.rs`. Move the cluster's `impl TopApp { … }`
   block. Move `RemoteDetailThumbnailState` and any private free fns the
   cluster owns. Add `use` statements to compile in isolation.
3. Add `mod search_dispatch;` declaration to `src/app.rs`.
4. Run `cargo check` and `cargo clippy -- -D warnings`. Fix any visibility
   issues by bumping items from private to `pub(super)` only as needed.
5. Repeat steps 2-4 for `src/app/breadcrumb.rs`.
6. Repeat steps 2-4 for `src/app/resize.rs`.
7. Search `tests/architecture_tests.rs` for guards that string-match
   `src/app.rs`. Retarget paths if the guard's intent now lives in a
   submodule. Do not relax guard logic.
8. Run the full test suite.

## Acceptance Criteria

- `src/app.rs` line count is in the 1,700-2,100 LOC range after the move.
- `src/app/search_dispatch.rs`, `src/app/breadcrumb.rs`, `src/app/resize.rs`
  exist and compile.
- No call site outside `src/app/*` had to change its `use` statements.
- `cargo fmt -- --check` passes.
- `cargo build` succeeds.
- `cargo test --lib` passes (no test edits except path retargets).
- `cargo test --test architecture_tests` passes.
- `cargo clippy -- -D warnings` passes.
- No new `#[allow(...)]` annotations were introduced.
- The app launches and the workspace renders the ContentList + Queue with
  divider; toolbar search round-trips; breadcrumb segments pop nav; pane
  resize drags. (Operator confirms; sandbox cannot init GPUI.)

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

## Prompt for lower-context coding model

You are implementing one bounded refactor task from a larger plan.

Implement only this task. Do not change behavior, do not change APIs, do not
touch unrelated code.

Read:
- `docs/adr/0050-post-adr-0048-module-decomposition.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
- `src/app.rs` in full
- `src/app/mod.rs` (and existing submodules under `src/app/`) to match the
  established submodule style
- `tests/architecture_tests.rs`

Goal:
- Move three handler clusters out of `src/app.rs` into new submodules:
  - `src/app/search_dispatch.rs` (search submit, result drill-down, Index
    async wiring, sync helpers, `RemoteDetailThumbnailState`)
  - `src/app/breadcrumb.rs` (ContentList breadcrumb handler + labelers)
  - `src/app/resize.rs` (content-pane fluid resize handlers + state
    accessors)

Constraints:
- Behavior-preserving move. No new methods. No signature changes.
- Submodules host `impl TopApp` blocks; don't introduce wrapping traits or
  types.
- No edits to view-model code, shells, composites, or the `discover/`
  module.
- Re-export discipline: callers outside `src/app/*` must not need to change
  their `use` statements.
- Bump items from private to `pub(super)` only as visibility demands. Do
  not introduce new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:
- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed (with LOC delta per file)
2. tests run + pass/fail counts
3. behavior changes (should be none)
4. visibility bumps (list each `pub(super)` you added)
5. deviations from the task
6. unresolved concerns

## Escalation Triggers

- An `impl TopApp` method calls a `fn` defined in the same file that is too
  cross-cutting to move cleanly (e.g., depends on three private fns spread
  across the file).
- An arch test guard's intent no longer maps to a single file after the
  move (e.g., a guard that asserts "all search dispatch lives in app.rs"
  must move, but the wording is ambiguous about whether `app/` qualifies).
- The line-count target cannot be hit without merging or splitting clusters
  in a way the ADR does not authorize.
