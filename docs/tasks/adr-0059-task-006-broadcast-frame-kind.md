# ADR 0059 Task 006: Broadcast Workspace Frame Kind

## Goal

Add the fifth workspace frame kind and its search scope. Wire the workspace
model, history, and chrome. No shell and no renderer.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `src/view_models/workspace/frame.rs`
- `src/view_models/workspace/mod.rs`
- `src/view_models/workspace/tests.rs`
- `src/view_models/workspace/nav.rs`
- `src/app/search_dispatch.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/workspace/frame.rs`
- `src/view_models/workspace/tests.rs`
- `src/app/search_dispatch.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`
- `src/view_models/broadcast.rs`
- `src/view_models/queue_now_playing.rs`
- `src/db.rs`, `src/api.rs`, `src/broadcast/**`

## Constraints

- `WorkspaceFrameKind::QueueNowPlaying` keeps its name and its meaning. The new
  kind is separate. The two can be active at the same time.
- The new kind is `Broadcast`. Do not name it with the words "now playing".
- Add `FrameSearchScope::BroadcastRows` so toolbar search has a defined meaning
  in the frame.
- Follow the existing detach eligibility and dock target choices for a
  non-content frame.
- Frame layout persistence must keep loading an older `config.toml` that does
  not know the new kind.

## Implementation Steps

1. Add `Broadcast` to `WorkspaceFrameKind`.
2. Add `BroadcastRows` to `FrameSearchScope`.
3. Update every exhaustive match on both enums. The compiler lists them.
4. Set the detach eligibility and the dock target for the new kind.
5. Update the frame chrome title and accessibility label for the new kind.
6. Update `src/app/search_dispatch.rs` so a search in the new frame routes to
   the new scope.
7. Update the workspace serialization path so an unknown kind in a stored layout
   falls back to the default layout instead of failing.
8. Add workspace model tests: add the frame, remove the frame, focus it,
   navigate back and forward, and load a layout written before this task.
9. Update the architecture guards that count or list frame kinds.

## Acceptance Criteria

- The workspace model holds five frame kinds.
- Toolbar search in the new frame resolves to `BroadcastRows`.
- A `config.toml` written before this change still loads.
- Queue and Broadcast can both be present in one layout.
- No shell, renderer, or service file changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test workspace --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- An exhaustive match sits in a module that this task lists under Do Not Touch.
- The stored layout format cannot ignore an unknown kind without a format
  change.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `src/view_models/workspace/frame.rs`
- `src/view_models/workspace/tests.rs`
- `src/app/search_dispatch.rs`

Goal:
- Add `WorkspaceFrameKind::Broadcast` and `FrameSearchScope::BroadcastRows`, and
  wire the workspace model.

Constraints:
- Do not rename or change `QueueNowPlaying`.
- Do not use the words "now playing" for the new kind.
- An older stored layout must still load.

Do not touch:
- `src/ui/**`, `src/view_models/broadcast.rs`, database, API, `src/broadcast/**`

Acceptance criteria:
- Five frame kinds, search routing works, older config loads.
- Workspace tests and architecture guards updated.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test workspace --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
