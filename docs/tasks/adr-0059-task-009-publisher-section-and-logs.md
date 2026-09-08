# ADR 0059 Task 009: Publisher Section Wiring And Log Panel

Status: Implemented, one visual gate open - 2026-09-08. Mechanical acceptance
met. Visual: the log panel lines are met; the failed-state reason and `Reset`
prominence stay OPEN, because the operator cannot reproduce a service failure
at this time. Establishes how a section composes into the Show screen mount.
Do it before 012, 014, and 015.

## Goal

Add a `Publisher` section to the `Show` screen mount, driven by the control
service through a runtime actor, plus a log panel.

**This packet establishes how a section composes into `Show`.** Packets 012,
014, and 015 follow the pattern it sets, so decide it deliberately.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/curator-workflow-ui-design-brief.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/broadcast/control.rs`
- `src/runtime/broadcast_observation.rs`
- `src/view_models/show.rs`
- `src/ui/shells/show.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/runtime/broadcast_service_watch.rs` (new)
- `src/runtime/mod.rs`
- `src/view_models/show.rs`
- `src/ui/shells/show.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/control.rs` (task 008 owns it)
- `src/api.rs`, `src/db.rs`
- `src/ui/shells/queue_now_playing.rs`

## How A Section Composes Into Show

`Show` is a screen mount, not a frame. ADR 0060. It has no navigation history,
no breadcrumb, and no content stack.

What exists today:

- `ShowPageVm { title, state_label, now_playing, empty_state, queue }`
- `ShowSlots` with three transport callbacks
- `render_show(vm, slots) -> ShowShell`, with `render_show_summary`,
  `render_now_playing_summary`, and `render_empty_summary`

A section is an optional field on `ShowPageVm` and a group of callbacks on
`ShowSlots`. An absent section renders nothing. It does not render as
unavailable, because ADR 0060 says a surface that cannot act is absent.

Keep the section list ordered and explicit. Do not build a generic section
container that later sections register into. Four known sections do not justify
a plugin model.

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
2. Add a `publisher: Option<PublisherSectionDisplay>` field to `ShowPageVm` and
   its callbacks to `ShowSlots`. Extend the projector in
   `src/view_models/show.rs` to read the service states. Add a `logs` display with the unit name, the line count, and an open
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

Mechanical:

- The view model exposes six service states, and no variant carries a raw
  transport error string.
- The `Failed` state carries a reason and marks `Reset` available and `Start`
  unavailable.
- A command success invalidates the actor snapshot, so the next projection
  reflects it without a remount.
- The view model carries an open and closed log-panel state, and the open state
  carries the journal text.
- A guard proves no screen or shell calls `systemctl` or `journalctl`.

Visual proof, operator only:

- The failed state names its reason and the `Reset` action is the obvious one.
- The log panel opens, is readable, and closes.

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
- `docs/plans/curator-workflow-ui-design-brief.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/broadcast/control.rs`
- `src/runtime/broadcast_observation.rs`
- `src/view_models/show.rs`, `src/ui/shells/broadcast.rs`

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
