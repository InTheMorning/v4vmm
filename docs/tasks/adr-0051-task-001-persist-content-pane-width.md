# ADR 0051 Task 001 — persist content pane width

## Goal

Persist `TopApp::content_pane_width` to `config.toml` under
`[workspace.layout]` and restore it on app boot, clamped to current workspace
pane bounds.

## Files To Inspect

- `docs/adr/0051-workspace-pane-width-persistence.md`
- `docs/plans/adr-0051-workspace-pane-width-persistence-phase-plan.md`
- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/app/resize.rs`
- `src/ui/layouts.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/app/resize.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/composites/split_pane.rs`
- `src/ui/shells/workspace.rs`
- Search, queue, library, and view-model modules
- Existing `[workspace_layout]` frame-layout schema

## Constraints

- Additive config only; older config files must keep loading.
- Unknown keys in `[workspace]` and `[workspace.layout]` must not fail parse.
- Malformed workspace layout preferences fall back to defaults without
  rejecting the whole config.
- Clamp loaded widths to `CONTENT_PANE_MIN_WIDTH..=CONTENT_PANE_MAX_WIDTH`.
- Persist once in `end_content_pane_resize`, not in `resize_content_pane`.
- Use `#[expect(...)]` rather than new `#[allow(...)]`.
- Do not commit.

## Implementation Steps

1. Add `WorkspaceConfig` and `WorkspaceLayoutPrefs` to `src/config.rs`.
2. Add defensive deserializers for the new nested workspace config, matching
   the existing malformed `[workspace_layout]` behavior.
3. Add `save_workspace_layout_prefs` that preserves unrelated TOML keys and
   writes `[workspace.layout].content_pane_width`.
4. Pass `cfg.workspace.layout` from bootstrap into `TopApp::new`.
5. Initialize `TopApp::content_pane_width` from a clamped helper.
6. In `end_content_pane_resize`, save the current width and surface failures
   through existing status plumbing.
7. Add unit tests and an architecture guard.

## Acceptance Criteria

- Missing `[workspace.layout]` uses `CONTENT_PANE_DEFAULT_WIDTH`.
- Out-of-range persisted values clamp on load.
- Resize move does not write config; resize end does.
- Saving pane prefs preserves unrelated top-level and nested TOML values.
- Existing `[workspace_layout]` frame layout behavior is unchanged.
- `cargo fmt -- --check` passes.
- `cargo check --quiet` passes.
- `cargo build --quiet` passes.
- `cargo test --lib --quiet` passes.
- `cargo test --test architecture_tests --quiet` passes.
- `cargo clippy --quiet -- -D warnings` passes.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo build --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from ADR 0051.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0051-workspace-pane-width-persistence.md`
- `docs/plans/adr-0051-workspace-pane-width-persistence-phase-plan.md`
- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/app/resize.rs`
- `src/ui/layouts.rs`
- `tests/architecture_tests.rs`

Goal:
- Persist `TopApp::content_pane_width` under `[workspace.layout]` and restore
  it on boot with clamping.

Constraints:
- Additive config only.
- Persist on resize end only.
- Do not touch `SplitPane`, workspace shell rendering, search, queue, library,
  or view-model modules.
- Do not alter the existing `[workspace_layout]` frame-layout schema.
- Add guards and focused tests.
- Do not add new `#[allow(...)]`.
- Do not commit.

Acceptance criteria:
- Missing or malformed pane prefs fall back to the default width.
- Out-of-range persisted widths clamp to current layout bounds.
- Saving pane prefs preserves unrelated config keys.
- The full listed test gate passes.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. visibility/API changes
5. deviations from task
6. unresolved concerns

## Escalation Triggers

- The nested `[workspace.layout]` TOML shape conflicts with the existing
  `[workspace_layout]` schema in a way that would require migration.
- GPUI `Pixels` cannot be converted to/from a persisted scalar without adding
  a new app-level helper.
- Persisting on resize end would require a render-shell or `SplitPane` change.
