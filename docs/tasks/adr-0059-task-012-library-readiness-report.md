# ADR 0059 Task 012: Library Broadcast Readiness Report

## Goal

Report the library tracks that carry no payment routes, so the operator finds a
dead payload before a show instead of after it.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/metadata.rs` (the `TXXX:MusicIndex Value Routes` writer)
- `src/audio_tags.rs`
- `src/application/queries/library.rs`
- `src/view_models/broadcast.rs`
- `src/db.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/application/queries/broadcast.rs` (new)
- `src/application/queries/mod.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/producer.rs`
- `src/metadata.rs` write paths
- `src/api.rs`

## Constraints

- The report reads. It writes no tag and changes no file.
- A track counts as ready when its local file carries a
  `TXXX:MusicIndex Value Routes` frame that parses as a non-empty array.
- Separate three results: ready, no route tag, and file missing. Do not collapse
  them. The provenance rule forbids one inferred answer.
- The scan reads files and blocks. Run it from a runtime actor or a command, not
  from a renderer.
- The count belongs in the `Source` section. The list belongs in the
  `ContentList` frame, not in a new list inside the broadcast frame.
- Add a CLI command first, as ADR 0017 requires.

## Implementation Steps

1. Add `src/application/queries/broadcast.rs` with a readiness query over the
   library tracks that have a local file.
2. Return a summary with the three counts and a list of the tracks that are not
   ready, with the reason for each.
3. Add `v4vmm broadcast readiness --json` to the CLI.
4. Extend the `Source` section display with a readiness label and a typed action
   that opens the list.
5. Route the action to the `ContentList` frame with a filter for the not-ready
   tracks.
6. Cache the summary in the actor snapshot. Do not scan on every render.
7. Add tests: a ready track, a track with no tag, a track with an empty array, a
   track with a missing file, and an empty library.
8. Add a guard that no renderer reads a file from disk for this report.
9. Capture a screenshot of the readiness label with a non-zero count and of the
   filtered list.

## Acceptance Criteria

- The three results stay separate in the summary and in the list.
- The CLI command prints the summary as JSON.
- The `Source` section shows the count and opens the list.
- No renderer reads a file.
- Screenshots exist for the label and the list.

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

- The `ContentList` frame cannot take a filter from another frame without a
  contract change.
- A full library scan is too slow to run in one pass and needs a stored column.
  Say so. A stored column needs a schema task.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/audio_tags.rs`, `src/application/queries/library.rs`
- `src/view_models/broadcast.rs`

Goal:
- Report library tracks with no payment routes, in the CLI and in the `Source`
  section.

Constraints:
- Read only. Three separate results: ready, no route tag, file missing.
- The scan blocks. Run it in an actor or a command, never in a renderer.
- Count in the `Source` section, list in the `ContentList` frame.
- CLI command first.

Do not touch:
- the producer, the metadata write paths, API

Acceptance criteria:
- Three results stay separate, CLI prints JSON, section opens the list.
- Guard blocks file reads in renderers.

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
