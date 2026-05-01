# ADR 0024 Task 004: Subscription And Download Slice

## Status

Planned.

## Task Goal

Migrate feed subscribe/unsubscribe, track download/remove, and library
membership workflows through ADR 0024 commands, events, local queries, and the
`DownloadManager` port.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/tasks/adr-0024-task-003-phase-2-checkpoint.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/library_service.rs`
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

## Implementation Steps

1. Define feed subscription and download command types/results.
2. Introduce a concrete adapter that implements `DownloadManager` by wrapping
   existing download behavior.
3. Route migrated screen workflows through `GpuiCommandRunner`.
4. Broadcast feed/library/download events through `ApplicationEventBus`.
5. Add local query refreshes needed by affected view-models.
6. Add architecture-test gates for migrated direct service calls.
7. Add tests for command success/failure, cancellation extension points, and
   emitted events.
8. Record CLI migration decisions for matching subscription/download commands.

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
