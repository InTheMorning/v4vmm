# ADR 0024 Task 005: Metadata And Feed Update Slice

## Status

Planned.

## Task Goal

Migrate MusicBrainz staging and feed update workflows through ADR 0024 commands,
events, and local queries while preserving metadata provenance.

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

1. Define metadata/feed update commands and results.
2. Add metadata/feed update event families.
3. Add local query APIs for staged MusicBrainz and feed update state.
4. Wrap existing metadata/feed services without moving them.
5. Migrate screen call sites through commands and event/query refresh.
6. Add architecture tests for migrated direct service calls.
7. Add tests for successful staging, failure, event emission, and provenance
   preservation.

## Acceptance Criteria

- Migrated metadata/feed update paths dispatch commands.
- Staged MusicBrainz/feed update snapshots are read through
  `ApplicationQueryService`.
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
