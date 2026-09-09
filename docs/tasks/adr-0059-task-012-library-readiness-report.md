# ADR 0059 Task 012: Library Broadcast Readiness Report

Status: Implemented - 2026-09-08. Mechanical and operator visual acceptance
met; the operator check confirmed the readiness count agrees with
`broadcast readiness --json`. Revised for ADR 0060 and ADR 0062.

## Goal

Report the library tracks that carry no payment routes, so the operator finds a
dead payload before a show instead of after it.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/curator-workflow-ui-design-brief.md`
- `docs/architecture/broadcast-chain.md`
- `src/metadata.rs` (the `TXXX:MusicIndex Value Routes` writer)
- `src/audio_tags.rs`
- `src/application/queries/library.rs`
- `src/view_models/show.rs`
- `src/db.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/application/queries/broadcast.rs` (new)
- `src/application/queries/mod.rs`
- `src/view_models/show.rs`
- `src/ui/shells/show.rs`
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
- **The count and the list live on different surfaces now.** The count belongs
  in the `Source` section of `Show`, because readiness is a pre-flight fact. The
  list belongs in `Music`, because a not-ready track is library content and
  ADR 0062 made `Music` the surface for library content.
- Opening the list switches section from `Show` to `Music` with a filter
  applied. Do not build a second list inside `Show`.
- Add a CLI command first, as ADR 0017 requires.

## Implementation Steps

1. Add `src/application/queries/broadcast.rs` with a readiness query over the
   local library tracks that have a local file. The report always describes the
   library this app owns and tagged. It never scans a remote host, because a
   difference between a local file and a remote copy is a synchronization
   problem and not a tagging problem.
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

Mechanical:

- The three results stay separate in the summary and in the list.
- The CLI command prints the summary as JSON.
- The view model carries the not-ready count and an action that opens the
  filtered list.
- A guard proves no renderer reads a file.

Visual proof, operator only:

- The count is legible in place and the action reaches the filtered list.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- Do not run the app. Write the operator visual check instead, as AGENTS.md
  requires.

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
- `docs/plans/curator-workflow-ui-design-brief.md`
- `src/audio_tags.rs`, `src/application/queries/library.rs`
- `src/view_models/show.rs`

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
