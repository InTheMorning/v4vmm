# ADR 0046 Task 009: QueueNowPlaying Page View Model

Status: Proposed - 2026-05-14.

## Goal

Define the GPUI-free `QueueNowPlayingPageVm` display contract used by
the QueueNowPlaying frame (task 010). First-slice scope: queue list,
transport controls (play/pause, skip previous, skip next), liveValue
device picker, volume slider.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/library.rs` (display-contract precedent)
- `src/playback*` for playback session shape
- `src/view_models.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/queue_now_playing.rs` (new)
- `src/view_models.rs` (module declaration)
- `tests/architecture_tests.rs`

## Do Not Touch

- Playback engine, mpv driver
- `src/db.rs`
- `src/ui/*`
- Library, search, settings shells

## Constraints

- Zero `gpui::*` imports.
- M-CANONICAL-DOCS on every public type.
- Display-ready fields only. No raw `Duration`, no `TrackRow`, no
  playback engine handles in the public VM surface.
- Transport state expressed as an enum (e.g., `Playing`, `Paused`,
  `Stopped`).
- Builder pattern if the VM grows beyond four constructor params.
- Unit tests cover empty queue, single-track playing, multi-track
  paused, no-device picker selection.

## Implementation Steps

1. Add `src/view_models/queue_now_playing.rs` and declare in
   `src/view_models.rs`.
2. Define `QueueRowDisplay { id, title, artist, duration_label,
   now_playing, a11y_label }`.
3. Define `TransportDisplay { play_pause_id, play_pause_label,
   play_pause_a11y_label, play_pause_state: TransportState,
   skip_previous: FrameChromeButtonDisplay-like,
   skip_next: same, disabled flags }`. Reuse existing button display
   types if a fit exists; otherwise add minimal types locally.
4. Define `LiveValueDeviceDisplay { picker_id, options:
   Vec<LiveValueDeviceOption>, selected_id }` where
   `LiveValueDeviceOption { id, label, a11y_label }`.
5. Define `VolumeDisplay { slider_id, level: f32, a11y_label }`.
6. Define `QueueNowPlayingPageVm { rows, transport, liveValue,
   volume }` plus a projector that consumes playback session state +
   queue state and returns a `QueueNowPlayingPageVm`.
7. Unit tests for empty queue (transport disabled), playing state,
   paused state, device-list passthrough.
8. Architecture guard asserts the module is GPUI-free and contains
   the listed display types.

## Acceptance Criteria

- [ ] Module compiles and documents public types.
- [ ] Display contract avoids `Duration`, `TrackRow`, playback engine
  types in public surface.
- [ ] Transport state modeled as enum.
- [ ] Unit tests cover empty/playing/paused/device passthrough.
- [ ] No `gpui` imports.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test queue_now_playing
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/library.rs` (display-contract precedent)
- `src/playback*`
- `tests/architecture_tests.rs`

Goal:
- Add `src/view_models/queue_now_playing.rs` with
  `QueueNowPlayingPageVm` and supporting display types.

Constraints:
- GPUI-free; no raw `Duration`/`TrackRow`/engine handles in public
  fields.
- Transport state enum.
- Documented public types.

Do not touch:
- Playback engine, mpv driver
- `src/db.rs`, `src/ui/*`
- Library, search, settings shells

Acceptance criteria:
- Module exists, public types documented, unit tests pass.
- Architecture guard records the contract.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test queue_now_playing`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Playback session shape does not expose enough state to project the
  VM without touching the engine (escalate before importing engine
  internals into the VM module).
