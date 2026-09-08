# ADR 0060 Task 002: Show Screen Mount And Queue Relocation

Status: Implemented - 2026-09-08. Mechanical and visual acceptance met.

## Goal

Add `Show` as a third app section and a third screen mount. Move the queue and
transport into it. `Show` is a container in this task. The broadcast sections
arrive when the revised ADR 0059 packets fill them.

## Files To Inspect

- `docs/adr/0060-workflow-surface-structure.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/adr/0048-content-list-frame-breadcrumb-search.md`
- `src/app.rs`, for `AppTab` and `WorkspaceScreenMount`
- `src/view_models/app_toolbar.rs`, for `AppToolbarTabKey`
- `src/view_models/queue_now_playing.rs`
- `src/ui/shells/queue_now_playing.rs`
- `src/app/queue_now_playing.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/app.rs`
- `src/view_models/app_toolbar.rs`
- `src/view_models/show.rs` (new)
- `src/ui/shells/show.rs` (new)
- `src/app/show.rs` (new)
- `src/ui/shells/mod.rs`, `src/view_models/mod.rs`
- `src/ui/shells/workspace.rs`
- `src/view_models/workspace/frame.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/view_models/queue_now_playing.rs` display contract. The queue moves. Its
  contract does not change in this task.
- `src/broadcast/**` and `src/runtime/broadcast_observation.rs`
- The `ContentList` navigation stack and breadcrumb behavior
- Settings

## Constraints

- **`Show` is a screen mount, not a workspace frame.** ADR 0060. It sits beside
  `Library` and `Settings` in `WorkspaceScreenMount`. Do not add a frame kind
  for it.
- The queue keeps its existing view model. This task relocates the surface that
  renders it, not the contract behind it.
- `QueueNowPlaying` keeps its name and meaning. It is local playback, and it is
  not broadcasting.
- The `Show` layout is built for reading at a distance: few panes, large type,
  no navigation history and no breadcrumb. A frame gives history and a content
  stack that this surface does not use.
- Activating a tab resets its own state, as ADR 0048 established for `Library`
  and `Settings`. `Show` follows the same rule.
- A stored `config.toml` written before this change still loads.
- No broadcast section in this task. The container only.

## Implementation Steps

1. Add `Show` to `AppTab`, `WorkspaceScreenMount`, and `AppToolbarTabKey`, with
   its label and accessibility label in the toolbar view model.
2. Add `src/view_models/show.rs` with `ShowPageVm`. First scope: the transport
   state, the queue rows projected from the existing queue view model, and an
   empty state for no active show.
3. Add `src/ui/shells/show.rs` with `render_show` and a slots builder for the
   transport callbacks, in the shape of the queue shell.
4. Add `src/app/show.rs` with the adapter that projects the view model and
   binds the slots.
5. Remove the `QueueNowPlaying` frame from the default workspace layout and
   render the queue inside `Show` instead. Keep the frame kind for now, and
   remove it in a later packet once nothing mounts it.
6. Route tab activation so `Show` mounts the new screen and leaves the
   `ContentList` stack untouched.
7. Add guards, marked situational and citing ADR 0060:
   - `Show` is a screen mount and no frame kind is named for it
   - the show shell imports no screen or backend module
   - the show view model is free of renderer types
8. Add a test that a stored layout written before this change still loads.
9. Capture a screenshot of `Show` with a queue and with the empty state.

## Acceptance Criteria

- Three app sections exist: `Music` or `Library` as currently labeled, `Show`,
  and `Settings`.
- `Show` renders the queue and the transport.
- The workspace no longer mounts a queue pane during curation.
- No frame kind is named for `Show`.
- An older `config.toml` still loads.
- The queue display contract is unchanged.

## Visual Acceptance

A person judges these. They are open until an operator runs the app outside a
headless session and reports each line. Never report them as met from a passing
mechanical run.

- `Show` presents the queue and the transport as one readable region, not as
  two unrelated blocks.
- Curation leaves no empty gap where the queue pane used to mount.
- Moving between the three sections keeps the window furniture stable.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- Do not run the app. Write the operator visual check instead, as AGENTS.md
  requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- The screen mount path cannot host a layout that differs from the frame lanes
  without a change to the workspace shell contract.
- Removing the queue pane from the default layout breaks a stored layout in a
  way the fallback does not cover.
- The queue view model turns out to depend on frame chrome. Report it rather
  than copying the chrome into `Show`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0060-workflow-surface-structure.md`
- `src/app.rs`, `src/view_models/app_toolbar.rs`
- `src/view_models/queue_now_playing.rs` and its shell and adapter

Goal:
- Add `Show` as a third app section and screen mount, and move the queue and
  transport into it.

Constraints:
- `Show` is a screen mount, never a workspace frame.
- The queue view model contract does not change.
- No breadcrumb and no navigation history on `Show`.
- No broadcast section in this task.
- An older `config.toml` still loads.

Acceptance criteria:
- Three sections, `Show` renders the queue, no queue pane during curation.
- Guards are situational and cite ADR 0060.
- Screenshots for queue and empty state.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured
5. deviations from task
6. unresolved concerns
