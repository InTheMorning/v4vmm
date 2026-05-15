# ADR 0046 Task 011: Phase 4 Guards and Visual

Status: Implemented - 2026-05-15.

## Goal

Lock Phase 4 invariants in architecture tests and record visual proof
for the QueueNowPlaying frame in light and dark themes. Verify
collapsing the queue frame preserves global playback status.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-009-queue-now-playing-page-vm.md`
- `docs/tasks/adr-0046-task-010-queue-now-playing-frame-shell.md`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

## Do Not Touch

- Production code paths
- Playback engine, mpv driver
- `src/db.rs`

## Constraints

- Guards map directly to ADR 0046 invariants 3 and 5 (toolbar global
  only; frame composite shared).
- Architecture tests should be named so their intent is obvious from
  `cargo test workspace_frame_phase_4_guards` style invocation.
- Visual proof must include: queue frame expanded with multi-track
  list, queue frame collapsed with toolbar status still visible,
  device picker open, volume slider mid-range, in both themes.

## Implementation Steps

1. Add architecture test asserting `src/app/tab_bar.rs` no longer
   contains queue, liveValue, or volume controls (string-level
   absence checks).
2. Add architecture test asserting
   `src/ui/shells/queue_now_playing.rs` uses `frame_shell` composite.
3. Add architecture test asserting compact Now Playing card retains
   play/pause and status (presence check on toolbar source).
4. Add architecture test asserting `QueueNowPlayingPageVm` is
   referenced by the QueueNowPlaying frame shell.
5. Update `docs/reviews/adr-0046-review-checklist.md` with Phase 4
   readiness section, gates, and visual-proof entries.

## Acceptance Criteria

- [x] Phase 4 architecture guards present and green.
- [x] Review checklist has Phase 4 section, gates, evidence pointers.
- [x] No production code modified.

## Implementation Notes

- Phase 4 guards now lock QueueNowPlaying VM and shell ownership,
  compact toolbar behavior, and scroll-chain boundaries.
- Final visual evidence and user confirmations are recorded in
  `docs/reviews/adr-0046-review-checklist.md`.

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
- `docs/tasks/adr-0046-task-009-queue-now-playing-page-vm.md`
- `docs/tasks/adr-0046-task-010-queue-now-playing-frame-shell.md`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

Goal:
- Add Phase 4 architecture guards and update the review checklist
  with Phase 4 readiness gates and visual-proof references.

Constraints:
- Guards map to ADR 0046 invariants.
- No production code edits.

Do not touch:
- Production code
- Playback engine

Acceptance criteria:
- Guards confirm queue/liveValue/volume gone from toolbar, frame
  shell shared, compact Now Playing retains status + play/pause.
- Review checklist updated.

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

- Guards need broad source-string baselines to stay green (signals
  tasks 009-010 left invariants partially implemented).
