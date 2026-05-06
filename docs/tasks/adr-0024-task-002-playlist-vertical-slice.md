# ADR 0024 Task 002: Playlist Vertical Slice

## Status

Completed.

## Task Goal

Migrate playlist create/delete/rename/reorder/append workflows through the ADR
0024 application layer.

## Completion Notes

- Implemented typed playlist commands for create, rename, delete, reorder,
  remove-at-position, and appending existing local tracks.
- Added local playlist query APIs on `ApplicationQueryService`.
- Wired `SearchApp` and `LibraryApp` to root `ApplicationServices` and
  `GpuiCommandRunner` for migrated playlist operations.
- Added playlist track-change events and architecture tests preventing screens
  from reintroducing direct `playlist_service` calls.
- Left subscription/download-backed "add to playlist" flows that call
  `library_service::subscribe_then_append_to_playlist` for Task 004, because
  they are not pure playlist mutations.
- CLI playlist paths were not migrated in this task; CLI parity remains a
  named follow-up for a later ADR 0024 slice.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/tasks/adr-0024-task-001-application-skeleton.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/playlist_service.rs`
- `src/cli.rs`
- `tests/common/mod.rs`

## Files Likely To Change

- `src/application/commands/playlist.rs`
- `src/application/queries/playlist.rs`
- `src/application/events/playlist.rs`
- `src/application/events/library.rs`
- `src/application/errors/command.rs`
- `src/application/application_services.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/cli.rs` only if the task explicitly migrates matching CLI paths
- `tests/architecture_tests.rs`
- Focused command/query tests

## Do Not Touch

- Subscription/download workflows.
- MusicBrainz/feed update workflows.
- Playback workflows.
- Database schema.
- Remote-only discovery/search.

## Constraints

- Use imperative command names such as `CreatePlaylist` and
  `AppendTracksToPlaylist`.
- Return typed `CommandOutcome<T>` values and broadcast playlist/library events
  through `ApplicationEventBus`.
- Keep view-models GPUI-free.
- Preserve playlist ordering and dedup semantics.
- Record whether related CLI playlist paths migrate in this task.

## Implementation Steps

1. Add playlist command types, results, and command handlers.
2. Add playlist local query APIs for playlist and playlist-track snapshots.
3. Add playlist/library application events for playlist mutations.
4. Wire handlers through `ApplicationServices`.
5. Migrate one playlist call path at a time from `library.rs` / `search.rs`.
6. Add architecture-test rules for migrated playlist service calls in screens.
7. Add focused tests for command success, command failure, emitted events, and
   query snapshots.
8. Decide and document whether CLI playlist commands use the same path now or a
   named later task.

## Acceptance Criteria

- Migrated playlist workflows dispatch commands instead of calling
  `playlist_service` directly from screens.
- Playlist snapshots are read through `ApplicationQueryService`.
- `ApplicationEventBus` can broadcast playlist/library events to app-level
  subscribers.
- Existing playlist UI and CLI behavior is preserved, except for intentionally
  documented internal routing changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test playlist`
- `cargo test --test architecture_tests`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Playlist behavior cannot be preserved without changing schema.
- Command events cannot update both Library and Discover state.
- `ApplicationServices` starts to require dynamic string lookup.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/playlist_service.rs`
- `tests/architecture_tests.rs`

Goal:
- Migrate playlist workflows through typed commands, local queries, and
  application events.

Constraints:
- Preserve playlist behavior.
- No subscription/download, metadata, or playback migration.
- No GPUI imports under `src/application/`.
- Use ADR 0024 command/result/event names.

Do not touch:
- Database migrations.
- Remote-only discovery/search.
- MusicBrainz/feed update code.
- Playback code.

Acceptance criteria:
- Screens no longer directly call migrated playlist service paths.
- Playlist command/query/event tests cover the migrated behavior.
- Architecture tests prevent regression.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test playlist`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
