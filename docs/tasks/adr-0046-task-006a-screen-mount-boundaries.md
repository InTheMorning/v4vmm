# ADR 0046 Task 006a: Screen Mount Boundaries

Status: Implemented - 2026-05-14.

## Goal

Define and implement the bounded mount boundary for existing Library, Search,
and Settings screens before the workspace shell renders them. This task
intentionally **does not** split Library/Search internals into separate
`SourceList`, `ContentList`, and `Detail` frame slots; both screens still own
their current split panes in this slice.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `src/app.rs`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/search.rs`
- `src/search/app_impl.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/app.rs`
- Possibly `src/ui/shells/workspace.rs` if a shell stub already exists
- `tests/architecture_tests.rs`

## Do Not Touch

- Library/Search split-pane internals
- Page VM internals
- `src/db.rs`, playback engine, mpv driver
- Toolbar global search behavior

## Constraints

- This is a mount-boundary task, not a screen decomposition task.
- Current Library and Search entities remain mounted whole.
- Do not duplicate Library/Search sidebar/detail render logic in the app shell.
- Do not introduce a second source-list implementation.
- Record the transitional rule in an architecture guard or task note:
  workspace rendering wraps whole existing screens until a later ADR/task
  extracts separate source/content/detail slots.

## Implementation Steps

1. Inspect where `TopApp::render` currently mounts Library, Search, and
   Settings.
2. Add a small projection/helper if needed so workspace render code can receive
   each current screen as a whole content slot.
3. Document in code comments or guard names that Phase 3 wraps existing screens
   whole.
4. Add or update an architecture guard preventing `src/app.rs` or
   `src/ui/shells/workspace.rs` from copying Library/Search split-pane
   internals.
5. Do not change visible behavior.

## Acceptance Criteria

- [x] Later workspace render can mount the existing Library/Search/Settings
  screens without internal screen refactors.
- [x] No Library/Search split-pane rendering is copied into app/workspace shell
  code.
- [x] Task 007 can render the workspace scaffold without pretending separate
  SourceList/ContentList/Detail slots already exist.

## Implementation Notes

- Added `WorkspaceScreenMount` in `src/app.rs` as the transitional Phase 3
  boundary for whole Library/Search/Settings screen mounts.
- Replaced inline active-tab child branching with
  `render_workspace_screen_mount(...)`, preserving visible behavior while
  giving Task 007 a single mount point for frame-shell wrapping.
- Added an architecture guard that records the whole-screen rule and blocks
  premature SourceList/ContentList/Detail splitting in the app mount boundary.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `src/app.rs`
- `src/library/app_impl.rs`
- `src/search/app_impl.rs`
- `tests/architecture_tests.rs`

Goal:
- Prepare the mount boundary so the workspace shell can wrap existing
  Library/Search/Settings screens whole.

Constraints:
- Do not split Library/Search internals.
- Do not duplicate split-pane render logic in the app shell.
- Do not change visible behavior.

Do not touch:
- Library/Search split-pane internals
- Page VM internals
- `src/db.rs`, playback engine
- Toolbar global search behavior

Acceptance criteria:
- Workspace render can consume whole screen mounts.
- Guards or comments make the transitional whole-screen wrapping explicit.
- No copied Library/Search render internals appear in workspace shell code.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Mounting whole screens requires extracting Library/Search internals now.
- The workspace shell needs direct access to Library/Search private split-pane
  functions.
