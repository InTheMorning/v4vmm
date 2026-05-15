# ADR 0046 Task 014: Detach/Dock Metadata

Status: Implemented - 2026-05-15.

## Goal

Add detach/dock eligibility metadata to the workspace model plus
deferred-error commands (`request_detach`, `request_dock`). Model
only. No UI exposes detach. No second OS window is created.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` (no detach UI)
- `src/app.rs` window code
- GPUI window primitives
- `src/db.rs`, playback engine

## Constraints

- Model-only. Commands return a deferred-error variant such as
  `Err(WorkspaceLayoutError::DetachDeferred)`.
- Eligibility expressed as an enum, not raw booleans on public types.
  Example: `FrameDetachEligibility { Detachable, NotDetachable }`.
- `M-CANONICAL-DOCS` on every new public type.
- Per-frame eligibility:
  - `SourceList`: `NotDetachable` (anchored to leading edge).
  - `ContentList`: `Detachable`.
  - `Detail`: `Detachable`.
  - `QueueNowPlaying`: `Detachable`.
- Dock targets similarly modeled (e.g.,
  `FrameDockTarget { Leading, Center, Trailing }`).
- Architecture guards assert no `src/ui/*` file references the detach
  commands or eligibility types.

## Implementation Steps

1. Add `FrameDetachEligibility` and `FrameDockTarget` enums.
2. Add `WorkspaceFrameKind::detach_eligibility(&self) ->
   FrameDetachEligibility`.
3. Add a `WorkspaceLayoutError::DetachDeferred` variant (or extend
   the existing error enum) signaling the command is recognized but
   not implemented.
4. Add `WorkspaceLayout::request_detach(frame_id) -> Result<(),
   WorkspaceLayoutError>` returning the deferred-error variant for
   any detachable frame and `NotDetachable` for ineligible frames.
5. Add `WorkspaceLayout::request_dock(frame_id, target) -> Result<(),
   WorkspaceLayoutError>` with the same return-shape rules.
6. Unit tests cover eligibility per frame kind, deferred-error
   variant returned for detachable frames, `NotDetachable` error
   returned for ineligible frames.
7. Architecture tests:
   - `src/view_models/workspace.rs` contains the new types and
     methods.
   - No `src/ui/*` file references
     `FrameDetachEligibility`, `FrameDockTarget`, `request_detach`,
     or `request_dock`.

## Acceptance Criteria

- [x] Detach/dock metadata and commands exist in workspace VM.
- [x] Commands return `DetachDeferred` or `NotDetachable` errors as
  appropriate; never panic.
- [x] Unit tests verify per-frame eligibility and command return
  shapes.
- [x] Architecture guard confirms no UI references the detach
  surface.

## Implementation Notes

- `src/view_models/workspace.rs` defines `FrameDetachEligibility`,
  `FrameDockTarget`, `WorkspaceFrameKind::detach_eligibility`,
  `WorkspaceLayout::request_detach`, and
  `WorkspaceLayout::request_dock`.
- Detachable frames return deferred model errors only. No UI,
  app-level command, GPUI window primitive, or second OS window is
  wired by this task.
- `tests/architecture_tests.rs` includes the Task 014 model-only guard
  so later work cannot expose detach/dock controls without a follow-up
  windowing ADR.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

Goal:
- Add detach/dock eligibility metadata and deferred-error commands to
  the workspace VM.

Constraints:
- Model only. No UI. No second OS window.
- Enum-based eligibility; never raw booleans on public types.
- Commands return `DetachDeferred` for eligible frames,
  `NotDetachable` for ineligible ones.

Do not touch:
- `src/ui/*`
- `src/app.rs` window code
- GPUI window primitives
- `src/db.rs`, playback engine

Acceptance criteria:
- Workspace VM compiles with detach/dock types and commands.
- Unit tests cover eligibility per frame kind and command returns.
- Architecture guard confirms no UI references the detach surface.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Modeling detach without GPUI window primitives requires importing
  window APIs (escalate; this task is model only).
