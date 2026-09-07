# ADR 0059 Task 011: mpv Drop-File Producer

## Goal

Write `musicindex.nowplaying/1` drop files for local `mpv` playback, so the
built-in player feeds the same chain as Mixxx.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `musicindex-live-publisher`: `docs/adr/0002-nowplaying-drop-file-contract.md`
- `musicindex-live-publisher`: `mixxx-now-playing/src/render.rs` (the reference
  producer)
- `src/playback.rs`, `src/playback_owner.rs`
- `src/metadata.rs` (the `TXXX:MusicIndex Value Routes` writer)
- `src/audio_tags.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/broadcast/producer.rs` (new)
- `src/broadcast/mod.rs`
- `src/playback_owner.rs` (call the producer on a state change)
- `src/app/bootstrap.rs` (clear on exit)
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/control.rs`, `src/broadcast/registry.rs`
- `src/api.rs`, `src/db.rs` schema
- `src/ui/**`

## Constraints

- **Read the payment routes from the tag on the audio file**, with the same
  method `mixxx-now-playing` uses. Do not build routes from
  `NowPlayingUpdate.value_block`. That field holds a generic conversion of the
  RSS `podcast:value` element, not the `PaymentRoute` shape.
- The drop file matches the contract exactly. The publisher repository owns it,
  and an unknown field is not permitted.
- Write a temporary file in the same directory, then rename it.
- **Pause counts as stopped.** Remove the drop file when playback pauses, so
  boosts fall back to the station route. The contract has no pause field.
- The panel must still show the paused state. Pause changes the file, not the
  display.
- **Remove the drop file when the app closes**, and warn the operator first,
  because closing the app ends the broadcast for this source.
- A missing tag is not an error. Write the file with an empty `value_routes`
  array and record the reason for the readiness report of task 012.

## Implementation Steps

1. Add `src/broadcast/producer.rs` with a `DropFileProducer` that holds the
   drop directory and the target name.
2. Define the payload struct with exactly the contract fields: `schema`,
   `target`, `artist`, `title`, `duration_secs`, `image`, `feed_guid`,
   `track_guid`, `value_routes`, `value_routes_source`.
3. Read the tags of the playing file: `MusicIndex Feed Guid`,
   `MusicIndex Track Guid`, `MusicIndex Image`, and
   `MusicIndex Value Routes`. Parse the routes as a JSON array.
4. Set `value_routes_source` to a value that names the tag as the source.
5. Add `publish(update)` that writes the file, and `clear()` that removes it.
6. Call `publish` when the session starts a track or changes track. Call
   `clear` when the session stops or pauses.
7. Add the drop directory to the `[broadcast]` config section. When it is not
   set, write nothing and stay silent. The producer is opt-in.
8. Add a warning before exit when the producer holds a live drop file, then
   clear the file.
9. Add golden tests: compare the written JSON against the field list of the
   contract, assert the split value stays a number in the drop file, and assert
   the file is removed on stop and on pause.
10. Add a test that the written object never has exactly the keys `event_id` and
    `metadata`.
11. Add a guard: only `src/broadcast/producer.rs` writes a file whose name ends
    with the drop-file suffix.

## Acceptance Criteria

- The written file matches the `musicindex.nowplaying/1` field list.
- Payment routes come from the file tag, not from `value_block`.
- Pause removes the file, and the panel still reports paused.
- Application exit warns and then removes the file.
- A track with no route tag writes an empty array and no error.
- The producer stays inactive when the drop directory is not configured.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast::producer --lib --quiet`
- `cargo test playback --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- The tag reader cannot return the `TXXX` frame values that the producer needs.
- The playback owner has no hook for a pause change without a contract change.
- The contract in the publisher repository disagrees with this task. The
  publisher repository wins. Stop and report the difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/architecture/broadcast-chain.md`
- `musicindex-live-publisher/docs/adr/0002-nowplaying-drop-file-contract.md`
- `musicindex-live-publisher/mixxx-now-playing/src/render.rs`
- `src/playback.rs`, `src/playback_owner.rs`, `src/audio_tags.rs`

Goal:
- Write `musicindex.nowplaying/1` drop files for local `mpv` playback.

Constraints:
- Read payment routes from the `TXXX:MusicIndex Value Routes` tag on the file.
  Do not use `NowPlayingUpdate.value_block`.
- Temporary file plus rename, in the same directory.
- Pause removes the file. Exit warns, then removes the file.
- Opt-in: no drop directory in config means the producer does nothing.

Do not touch:
- the control service, the registry service, API, database schema, UI

Acceptance criteria:
- Golden test against the contract field list.
- File removed on stop, on pause, and on exit.
- Missing tag writes an empty array without an error.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast::producer --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
