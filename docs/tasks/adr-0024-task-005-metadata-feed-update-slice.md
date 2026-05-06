# ADR 0024 Task 005: Metadata And Feed Update Slice

## Status

Completed.

## Task Goal

Migrate MusicBrainz staging and feed update workflows through ADR 0024 commands,
events, and local queries while preserving metadata provenance.

## Progress Notes

- Feed update checks now dispatch ADR 0024 commands from `LibraryApp`:
  single-feed stale checks, subscribed-feed stale scans, and applying staged
  feed updates.
- `ApplicationQueryService` now owns the local subscribed-feed stale-check
  snapshot query used before bulk remote checks.
- Applying feed updates emits feed, library, and metadata events so app-level
  refresh paths can react to tag/feed changes.
- Single-track Library `MusicBrainz` lookup and staging now dispatch metadata
  commands instead of calling `feed_service` directly from the screen.
- Album batch `MusicBrainz` still owns its GPUI progress loop in `library.rs`,
  but album release lookup and per-track staging now route through metadata
  commands.
- Discover/Search `MusicBrainz` lookup remains deferred as remote-only lookup
  unless a later task treats its inspector state as command lifecycle state.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/metadata_service.rs`
- `src/feed_service.rs`
- `src/musicbrainz.rs`
- `src/metadata.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/common/mod.rs`

## Files Likely To Change

- `src/application/commands/metadata.rs`
- `src/application/commands/feed.rs`
- `src/application/queries/metadata.rs`
- `src/application/events/metadata.rs`
- `src/application/events/feed.rs`
- `src/application/errors/command.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`
- Focused metadata/feed command tests

## Do Not Touch

- Playback migration.
- Remote-only discovery/search that does not stage local state.
- Database schema unless a separate migration task is approved.
- Metadata inference rules not already approved by ADR or plan.

## Constraints

- MusicBrainz lookup is a command because it performs network I/O and stages
  local metadata.
- Preserve source facts and surface conflicts; do not add hidden inference.
- Use local queries for staged MusicBrainz status and feed update state.
- GPUI screens must not call migrated metadata/feed update service paths.

## Implementation Steps

1. Done: define metadata/feed update commands and results. Feed update,
   single-track `MusicBrainz`, album release lookup, and staging commands are
   present.
2. Done: add metadata/feed update event families. Feed apply emits
   metadata/feed/library events; single-track staging emits metadata events.
3. Done: add local query APIs for feed update state. Feed stale-check
   rows are query-backed; staged `MusicBrainz` remains in the library
   view-model snapshot because it is screen-local transient state, not a
   durable read model.
4. Done: wrap existing metadata/feed services without moving them.
5. Done: migrate screen call sites through commands and event/query
   refresh.
6. Done: add architecture tests for migrated direct service calls.
7. Done: add tests for successful staging, failure, event emission, and
   provenance preservation.

## Acceptance Criteria

- Migrated metadata/feed update paths dispatch commands.
- Feed update snapshots that come from local persistence are read through
  `ApplicationQueryService`.
- Staged MusicBrainz transient state remains GPUI-free view-model data until a
  durable read model exists.
- Metadata provenance behavior is preserved.
- Architecture tests prevent migrated direct service-call regression.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test musicbrainz`
- `cargo test metadata`
- `cargo test --test architecture_tests`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Existing metadata behavior is ambiguous or undocumented.
- A workflow would require hidden metadata inference.
- Remote-only lookup behavior does not stage local state.
- A schema change appears necessary.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `src/application/**`
- `src/library.rs`
- `src/search.rs`
- `src/metadata_service.rs`
- `src/feed_service.rs`
- `src/musicbrainz.rs`
- `src/metadata.rs`

Goal:
- Migrate MusicBrainz staging and feed update workflows through application
  commands, events, and local queries.

Constraints:
- Preserve metadata provenance.
- Do not introduce hidden inference.
- No playback migration.
- No remote-only query abstraction.

Do not touch:
- Database schema unless separately approved.
- Playback code.

Acceptance criteria:
- Migrated screens dispatch commands.
- Local staged/feed snapshots are read through `ApplicationQueryService`.
- Tests and architecture gates cover the slice.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test musicbrainz`
- `cargo test metadata`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
