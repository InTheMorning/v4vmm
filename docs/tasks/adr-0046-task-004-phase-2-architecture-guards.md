# ADR 0046 Task 004: Phase 2 Architecture Guards

Status: Proposed - 2026-05-14.

## Goal

Lock in Phase 2 invariants as architecture-test guards: frame nav
state lives in `src/view_models/workspace.rs`, it remains GPUI-free,
inspectors no longer own cross-frame navigation, and
`InspectorFrame.origin` / `InspectorOrigin` no longer carries playlist-return
navigation.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `docs/tasks/adr-0046-task-003-retire-inspector-back-button.md`
- `tests/architecture_tests.rs`
- `src/view_models/workspace.rs`
- `src/library.rs`
- `src/ui/shells/library/track_detail_metadata.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`

## Do Not Touch

- Production code paths
- Frame nav state implementation
- `src/db.rs`, playback engine

## Constraints

- Guards must map directly to ADR 0046 invariants 1, 2, and 4.
- Prefer narrow source-string guards over broad baselines.
- Tests added in this task must be named so their intent is obvious
  from `cargo test workspace_frame_phase_2_guards` style invocation.

## Implementation Steps

1. Add a test that asserts `src/view_models/workspace.rs` contains
   no `use gpui` or `gpui::` strings.
2. Add a test that asserts `src/library.rs` does not contain
   `pub(crate) origin: Option<InspectorOrigin>` after the field is
   retired, or otherwise asserts the field is not read by track-detail
   renderers if a non-navigation origin survives.
3. Add a test that asserts
   `src/ui/shells/library/track_detail_metadata.rs` does not contain
   `playlist_return_display`, `LibraryTrackPlaylistReturnDisplay`,
   `return_to_playlist`, or `Back to Playlist`.
4. Add a test that asserts `src/view_models/workspace.rs` contains
   `WorkspaceFrameId`, `WorkspaceFrameKind`, `WorkspaceFrameState`,
   `WorkspaceLayout`, `FrameNavigationState`, and
   `FrameNavigationEntry`.
5. Add a test that asserts `src/library/app_impl.rs` references
   `FrameNavigationState` (frame nav state is wired in).

## Acceptance Criteria

- [ ] All four guards are present in `tests/architecture_tests.rs`.
- [ ] All required checks are green.
- [ ] No production code path was modified by this task.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `docs/tasks/adr-0046-task-003-retire-inspector-back-button.md`
- `tests/architecture_tests.rs`

Goal:
- Add architecture-test guards locking in Phase 2 invariants.

Constraints:
- Guards map directly to ADR 0046 invariants.
- Source-string guards, not broad baselines.
- No production code edits.

Do not touch:
- Any `src/*` production module

Acceptance criteria:
- Guards confirm GPUI-free workspace module, absent
  absent inspector-owned origin navigation, absent inspector back button, present
  workspace types, present frame nav wiring.
- Required checks are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Guards require modifying production code to keep clippy or test
  green (signals tasks 001-003 were incomplete).
