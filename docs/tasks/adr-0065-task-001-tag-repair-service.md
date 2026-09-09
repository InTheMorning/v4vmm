# ADR 0065 Task 001: Payment Route Tag Repair Service

Status: Ready - 2026-09-08. Backend and CLI only. Task 002 gives it a surface.

## Goal

Write the embedded `TXXX:MusicIndex Value Routes` tag for a track that lacks it,
without asking whether the feed changed upstream.

## Files To Inspect

- `docs/adr/0065-payment-route-tag-repair.md`
- `src/feed_service.rs`, for `check_feed_staleness` and `refresh_stale_feed`,
  the path this task does not use
- `src/metadata_service.rs`, for `id3_edits_for_track_context`
- `src/audio_tags.rs`, for `write_id3v24_edits`
- `src/api.rs`, for `payment_routes` and the feed-level fallback at line 208
- `src/application/queries/broadcast.rs`, for the readiness states
- `src/application/commands/metadata.rs`, for the command precedent
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/application/commands/payment_routes.rs` (new)
- `src/application/commands/mod.rs`
- `src/application/queries/broadcast.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/feed_service.rs`. `Check all feeds` keeps its meaning, and this task adds
  no repair to it.
- `src/ui/**`, `src/view_models/**`. Task 002 owns the surface.
- `src/broadcast/**`, `src/runtime/**`
- `write_id3v24_edits` and `id3_edits_for_track_context`. Reuse them unchanged.

## Constraints

- **Gate on the file, never on feed staleness.** A track qualifies when its file
  lacks the tag. This is an ADR 0065 invariant, and it is the defect that record
  exists to fix.
- **The repair runs only when asked.** No start-up hook, and no call from a
  refresh. ADR 0065 separates a step that writes a file from one that reads.
- Build the client through `api::Client`. ADR 0058 forbids a second construction
  site.
- Request `payment_routes` in the include list, and take the feed-level fallback
  that `src/api.rs` already applies when a track carries none.
- A repair ends in one of three outcomes, and the caller can tell them apart:
  `Repaired`, `NoRoutesUpstream`, `Failed` with a reason.
- **`NoRoutesUpstream` is not a failure.** It is the answer that this app can not
  fix the track, and a later run must not retry it as though it might.
- Never write a file when the routes are empty. An empty tag reads as ready to
  the readiness check and pays nobody.
- The repair is safe to run twice. A track that is already ready is skipped.

## Implementation Steps

1. Add `src/application/commands/payment_routes.rs` with a command that repairs
   one track:
   - read the track row and resolve its file
   - return early when the file already carries a non-empty tag
   - fetch the MusicIndex track with `payment_routes` in the include list
   - when no routes reach the track, return `NoRoutesUpstream`
   - build edits with `id3_edits_for_track_context` and write them with
     `write_id3v24_edits`
   - return `Repaired` with the number of frames written
2. Add a command that repairs every not-ready track, and returns one outcome for
   each. It stops on no single failure, and reports all three counts.
3. Add `BroadcastReadinessState::NoRoutesUpstream` and count it in the summary,
   so the readiness report can hold the answer after a repair runs.
4. Extend `readiness_detail_label` in `src/view_models/show.rs` to name that
   count, so the `Source` card says which tracks a publisher must fix. This is
   the one view-model change this task makes.
5. Add `v4vmm broadcast repair-routes --json` and
   `v4vmm broadcast repair-routes <track-id> --json` to the CLI.
6. Document both commands in `docs/runbooks/workflows.md`.
7. Add unit tests with a stub API client:
   - a track with routes upstream is repaired, and the tag is written
   - a track with no routes upstream returns `NoRoutesUpstream` and writes no
     file
   - a track that already carries a tag is skipped
   - a write failure returns `Failed` with a reason
   - a second run over the same library repairs nothing more
8. Add a guard: `src/feed_service.rs` gains no tag repair, and no repair runs
   outside a command.

## Acceptance Criteria

Mechanical, proved by a test:

- A track lacking the tag is repaired whatever its feed staleness says.
- A track with no routes upstream returns `NoRoutesUpstream` and no file is
  written.
- An empty route set never reaches a file.
- A track that already carries the tag is skipped.
- Two runs give the same result as one.
- The readiness summary counts `NoRoutesUpstream` separately from `NoRouteTag`.
- The guard blocks a tag repair inside `src/feed_service.rs`.
- `v4vmm broadcast repair-routes --json` prints the three counts.

Visual, operator only:

- None. This task changes no renderer beyond the `Source` card detail text, and
  task 002 owns the visual check for that.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

Do not run the app. Write the operator visual check instead, as AGENTS.md
requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check

## Escalation Triggers

- `id3_edits_for_track_context` produces no Value Routes edit for a track that
  has routes upstream. Report it. That is a second defect in the same chain, and
  this task does not cover it.
- The stored track context lacks a fact the fetch needs, so the repair can not
  identify the track at MusicIndex.
- A non-MP3 file can not carry the frame. Report the format and the count.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0065-payment-route-tag-repair.md`
- `src/metadata_service.rs`, `src/audio_tags.rs`, `src/api.rs`
- `src/application/commands/metadata.rs` for the command precedent

Goal:
- A command that writes the payment-route tag for a track that lacks it, gated
  on the file and not on feed staleness.

Constraints:
- Never gate on feed staleness. Never run without an operator action.
- Reuse `id3_edits_for_track_context` and `write_id3v24_edits`.
- Build the client through `api::Client`.
- Three outcomes: `Repaired`, `NoRoutesUpstream`, `Failed`.
- Never write an empty route set.

Do not touch:
- `src/feed_service.rs`, `src/ui/**`, `src/view_models/**` beyond the `Source`
  card detail text, `src/broadcast/**`, `src/runtime/**`

Acceptance criteria:
- A track with no routes upstream writes no file and says so.
- A track already carrying the tag is skipped.
- A guard blocks a tag repair inside `src/feed_service.rs`.

Test commands:
- `cargo fmt -- --check`
- `cargo test --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. operator visual check
