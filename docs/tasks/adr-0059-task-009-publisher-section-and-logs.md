# ADR 0059 Task 009: Publisher Section Wiring And Log Panel

## Goal

Connect the control service to the `Publisher` section through a runtime actor,
and add a log panel that opens from a button.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/broadcast/control.rs`
- `src/runtime/broadcast_observation.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/app/broadcast.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/runtime/broadcast_service_watch.rs` (new)
- `src/runtime/mod.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/app/broadcast.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/control.rs` (task 008 owns it)
- `src/api.rs`, `src/db.rs`
- `src/ui/shells/queue_now_playing.rs`

## Constraints

- `systemctl` blocks. The screen must never call it. Put the read loop in a
  runtime actor, as ADR 0040 requires. The architecture guards already block
  `cx.spawn` in a screen.
- A command such as start or stop runs through the existing async command
  runner, not on the render thread.
- **The panel must show `Failed` as its own state with the reason.** Without it,
  the start button appears to do nothing.
- The log panel is a panel, not a modal sheet. Apple macOS guidance asks for
  less modality and fewer nested levels.
- The log text is untrusted output. Render it as text. Do not parse it into
  actions.
- After a successful command, the current view must update in place. Do not ask
  the operator to leave the frame and return.

## Implementation Steps

1. Add `src/runtime/broadcast_service_watch.rs`, an actor that calls
   `control::show` for both units on an interval and publishes the states over
   a `watch` channel. Reuse the observation actor shape.
2. Extend the projector in `src/view_models/broadcast.rs` to read the service
   states. Add a `logs` display with the unit name, the line count, and an open
   state.
3. Wire the start, stop, and reset slots to the async command runner.
4. On a command success, invalidate the actor snapshot so the section updates
   in place.
5. Add the log panel to the shell. It opens from the logs action and closes from
   its own control.
6. Show the failure reason in the section when the state is `Failed`, and make
   `Reset` the available action.
7. Add view-model tests for: active, inactive, failed with a reason, not
   installed, and the log panel open state.
8. Add a guard that no screen or shell file calls `systemctl` or `journalctl`.
9. Capture a screenshot of a failed unit with its reason and of the open log
   panel.

## Acceptance Criteria

- The section shows six states and never shows a raw transport error.
- A failed unit shows its reason and offers `Reset`.
- Start, stop, and reset update the section without a frame change.
- The log panel opens and closes, and holds the journal text.
- No blocking call runs on the render thread.
- Screenshots exist for the failed state and the open log panel.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo run` for the visual check

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- The async command runner cannot carry a command that returns text.
- The current-view update needs a change in the workspace frame contract.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/broadcast/control.rs`
- `src/runtime/broadcast_observation.rs`
- `src/view_models/broadcast.rs`, `src/ui/shells/broadcast.rs`

Goal:
- Add a service watch actor, wire start, stop, reset, and add a log panel.

Constraints:
- No blocking call from a screen. Use a runtime actor and the async command
  runner.
- `Failed` is shown with its reason and offers `Reset`.
- The log panel is a panel, not a modal. Log text is rendered, never parsed.
- The section updates in place after a command succeeds.

Do not touch:
- `src/broadcast/control.rs`, API, database, the queue shell

Acceptance criteria:
- Six states render, failed shows a reason, commands update in place.
- Guard blocks `systemctl` and `journalctl` in screens and shells.
- Screenshots for the failed state and the log panel.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured
5. deviations from task
6. unresolved concerns
