# ADR 0024 Task 006: Playback Slice

## Status

Planned.

## Task Goal

Route playback transport actions and playback snapshots through the ADR 0024
application layer while preserving the existing playback owner/driver boundary.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `src/application/**`
- `src/app.rs`
- `src/playback.rs`
- `src/playback_owner.rs`
- `src/playback_driver/**`
- `src/ui/composites/now_playing_bar.rs`
- `tests/common/mod.rs`

## Files Likely To Change

- `src/application/commands/playback.rs`
- `src/application/queries/playback.rs`
- `src/application/events/playback.rs`
- `src/application/errors/command.rs`
- `src/application/ports/mod.rs`
- Potential playback adapter port if needed
- `src/app.rs`
- `src/playback.rs`
- `src/playback_owner.rs`
- Playback-focused tests
- `tests/architecture_tests.rs`

## Do Not Touch

- Playlist, subscription/download, or metadata migration except for shared event
  plumbing.
- Playback driver internals unless required to expose a narrow adapter.
- Database schema.

## Constraints

- Use intuitive command names: `PlayTrack`, `PausePlayback`,
  `ResumePlayback`, `StopPlayback`, `SeekPlayback`, `SetPlaybackVolume`.
- Do not use `StartPlayback` for playing a track.
- Preserve current `PlaybackOwner<D>` and driver behavior unless a later ADR
  changes that architecture.
- `PlaybackSnapshot` is a local query.
- Low-level process supervision stays in the existing playback owner/driver
  boundary.

## Implementation Steps

1. Define playback command types/results and `PlaybackSnapshot`.
2. Add playback events.
3. Wrap existing playback calls through application handlers.
4. Route root/now-playing UI actions through `GpuiCommandRunner`.
5. Add local playback query refreshes.
6. Add tests for command naming, event emission, and snapshot reads.
7. Add architecture-test gates for migrated playback direct calls.

## Acceptance Criteria

- Migrated playback UI actions dispatch playback commands.
- Playback snapshot reads route through `ApplicationQueryService`.
- Existing playback behavior is preserved.
- `StartPlayback` is not used for track playback.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test playback`
- `cargo test --test architecture_tests`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Playback owner/driver behavior must change materially.
- Command naming becomes ambiguous.
- A new playback adapter port is needed but cannot be kept narrow.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `src/application/**`
- `src/app.rs`
- `src/playback.rs`
- `src/playback_owner.rs`
- `src/playback_driver/**`
- `src/ui/composites/now_playing_bar.rs`

Goal:
- Route playback commands and snapshots through ADR 0024 boundaries.

Constraints:
- Use the ADR-approved playback command names.
- Preserve current playback owner/driver behavior.
- Do not migrate unrelated workflows.

Do not touch:
- Database schema.
- Playlist/subscription/metadata migration except shared event plumbing.

Acceptance criteria:
- Playback actions dispatch commands.
- Playback snapshots use local queries.
- Tests and architecture gates cover the slice.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test playback`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
