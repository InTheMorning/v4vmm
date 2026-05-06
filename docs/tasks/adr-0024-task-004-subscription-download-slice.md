# ADR 0024 Task 004: Subscription And Download Slice

## Status

Completed 2026-05-01.

## Task Goal

Migrate feed subscribe/unsubscribe, track download/remove, and library
membership workflows through ADR 0024 commands, events, local queries, and the
`DownloadManager` port.

This task also owns the remaining screen add-to-playlist flows that currently
route through `library_service::subscribe_then_append_to_playlist`, because
those paths may subscribe, download, and append in one workflow.

## Progress Notes

- App-level consumption of `ApplicationEventBus` is wired through
  `GpuiEventBridge`, `GpuiCommandRunner`, and `TopApp` draining.
- Library, playlist, feed, and download application events now refresh the
  shared library/discover surfaces through a single app-level path.
- Feed unsubscribe, track remove, and local track library-membership toggles
  now dispatch ADR 0024 commands from screens.
- Feed subscribe, track subscribe/download, and subscribe-then-append playlist
  workflows now dispatch ADR 0024 commands from screens.
- `ServiceDownloadManager` wraps the current subscribe/download service
  functions behind the `DownloadManager` port.
- Architecture tests prevent screens from reintroducing the migrated direct
  feed/track remove, subscribe/download, and subscribe-then-append service
  calls.
- CLI decision: no CLI feed subscribe, track download, or subscribe-then-append
  workflows exist in this codebase, so this slice has no matching CLI path to
  migrate.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/tasks/adr-0024-task-003-phase-2-checkpoint.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/library_service.rs`
- `src/playlist_service.rs`
- `src/audio_format.rs`
- `src/audio_tags.rs`
- `tests/common/mod.rs`

## Files Likely To Change

- `src/application/commands/feed.rs`
- `src/application/commands/download.rs`
- `src/application/queries/library.rs`
- `src/application/events/feed.rs`
- `src/application/events/library.rs`
- `src/application/ports/download_manager.rs`
- `src/application/errors/command.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`
- Focused command/port tests

## Do Not Touch

- MusicBrainz staging/feed update migration.
- Playback migration.
- Remote-only discovery/search.
- Database schema unless explicitly approved by a separate migration task.

## Constraints

- Preserve existing download, tag-write, and library-membership behavior.
- Commands must use `CommandContext` for operation id/cancellation state.
- Commands depend on `DownloadManager`, not a concrete download implementation.
- Source facts and tag provenance must not be discarded.
- GPUI screens must not call migrated subscription/download service paths.
- App-level event consumption must be wired before replacing the remaining
  cross-view refresh paths.

## Implementation Steps

1. Completed: wire `ApplicationEventBus` to GPUI presentation refresh through
   `GpuiEventBridge` or an equivalent app-level subscriber.
2. Completed: define feed subscription and download command types/results.
3. Completed: introduce a concrete adapter that implements `DownloadManager`
   by wrapping existing subscribe/download behavior.
4. Completed: migrate `library_service::subscribe_then_append_to_playlist`
   callers from screens into a command that coordinates subscription/download
   and playlist append behavior.
5. Completed: route migrated screen workflows through `GpuiCommandRunner`.
6. Completed: broadcast feed/library/download/playlist events through
   `ApplicationEventBus`.
7. Completed: affected view-model refreshes are driven by existing local query
   refresh paths through the app-level event bridge.
8. Completed: add architecture-test gates for migrated direct service calls.
9. Completed: add tests for command success, cancellation extension points, and
   emitted events.
10. Completed: record CLI migration decisions for matching
    subscription/download commands.

## Acceptance Criteria

- Migrated screen paths do not call subscription/download services directly.
- `DownloadManager` is the command dependency for download work.
- Commands expose operation id and cancellation extension points.
- Existing behavior is preserved.
- Architecture tests prevent direct-call regression for migrated paths.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test download`
- `cargo test subscribe`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Existing download code cannot be wrapped by a `DownloadManager` port.
- Cancellation cannot be represented by `CommandContext`.
- A schema change appears necessary.
- Tag/source-fact provenance would change.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/tasks/adr-0024-task-003-phase-2-checkpoint.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/library_service.rs`

Goal:
- Migrate subscription/download workflows through ADR 0024 boundaries.

Constraints:
- Use `DownloadManager`.
- Use `CommandContext` for operation/cancellation.
- Preserve existing behavior and provenance.
- Do not migrate metadata/feed-update or playback workflows.

Do not touch:
- Database schema unless separately approved.
- Remote-only discovery/search.
- Playback code.

Acceptance criteria:
- Migrated screens dispatch commands.
- Download commands depend on `DownloadManager`.
- Tests and architecture gates cover the slice.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test download`
- `cargo test subscribe`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
