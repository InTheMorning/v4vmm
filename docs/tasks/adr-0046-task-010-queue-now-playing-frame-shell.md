# ADR 0046 Task 010: QueueNowPlaying Frame Shell

Status: Implemented - 2026-05-15.

## Goal

Render the trailing QueueNowPlaying frame using
`QueueNowPlayingPageVm` (task 009) and `frame_shell` (task 006).
Reduce the top-toolbar Now Playing card to compact status plus
minimal transport. Detailed queue, liveValue picker, and volume
controls live in the frame.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `docs/tasks/adr-0046-task-009-queue-now-playing-page-vm.md`
- `src/view_models/queue_now_playing.rs`
- `src/ui/shells/workspace.rs`
- `src/ui/shells/library/track_row.rs` (if track-row primitive
  reusable)
- `src/app/tab_bar.rs`
- `src/ui/composites/frame_shell.rs`

## Files Likely to Change

- `src/ui/shells/queue_now_playing.rs` (new)
- `src/ui/shells/workspace.rs` (mount the frame)
- `src/app/tab_bar.rs` (reduce Now Playing card)
- `src/view_models/library.rs` (if a compact-Now-Playing display
  needs slimming)

## Do Not Touch

- Playback engine, mpv driver
- `src/db.rs`
- Library/Search/Settings page logic

## Constraints

- Apple HIG: play/pause is a single toggle icon, not two buttons.
  Skip glyphs are standard. Volume slider uses existing slider
  primitive. Device picker uses existing menu primitive.
- Frame uses `frame_shell` for its chrome; queue contents render in
  the content slot.
- Toolbar Now Playing card retains: track title/artist text, single
  transport toggle (play/pause), and progress (if currently present).
  All other controls move into the frame.
- Reuse existing track-row primitive where it fits; otherwise add a
  minimal `queue_row` primitive co-located with the shell.
- No raw glyph/color/spacing literals; consume tokens.

## Implementation Steps

1. Add `src/ui/shells/queue_now_playing.rs` exporting
   `render_queue_now_playing(vm, slots)`.
2. Map `QueueNowPlayingPageVm.rows` to a list of rendered rows. If a
   shared track-row primitive fits, reuse; otherwise inline a queue
   row component limited to this shell.
3. Render transport controls beneath the queue list, with play/pause
   toggling based on `TransportState`.
4. Render liveValue device picker and volume slider via existing
   primitives.
5. Wire the QueueNowPlaying frame in `src/ui/shells/workspace.rs` to
   call this shell.
6. In `src/app/tab_bar.rs`, slim the Now Playing card to status +
   play/pause + progress. Remove anything that moved to the frame.

## Acceptance Criteria

- [x] QueueNowPlaying frame renders queue list, transport, liveValue
  picker, and volume.
- [x] Toolbar Now Playing card is reduced to compact status +
  play/pause + progress.
- [x] Frame uses `frame_shell` composite for its chrome.
- [x] No playback engine modifications.
- [x] No raw glyph/color/spacing literals.

## Implementation Notes

- The QueueNowPlaying frame is rendered through shared workspace frame
  chrome and keeps detailed queue/output controls out of the global
  toolbar.
- Phase 4 architecture guards and final visual evidence are recorded in
  `docs/reviews/adr-0046-review-checklist.md`.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `docs/tasks/adr-0046-task-009-queue-now-playing-page-vm.md`
- `src/view_models/queue_now_playing.rs`
- `src/ui/shells/workspace.rs`
- `src/app/tab_bar.rs`
- `src/ui/composites/frame_shell.rs`

Goal:
- Render QueueNowPlaying frame from `QueueNowPlayingPageVm` and
  reduce toolbar Now Playing card to compact status + minimal
  transport.

Constraints:
- HIG-compliant transport glyphs; play/pause toggle, not split
  buttons.
- Reuse `frame_shell` for chrome.
- Reuse existing primitives for slider, menu, track row.

Do not touch:
- Playback engine, mpv driver
- `src/db.rs`
- Library/Search/Settings page logic

Acceptance criteria:
- Queue list, transport, liveValue picker, volume render inside the
  frame.
- Toolbar Now Playing is reduced.
- No engine modifications.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Existing track-row primitive cannot host queue rows without a
  signature change.
- Toolbar Now Playing card cannot be slimmed without restructuring
  global search layout from ADR 0043 (escalate first).
