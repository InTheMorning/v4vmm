# ADR 0059 Task 004: Relay Observation Actor

## Goal

Add a runtime actor that reads the relay snapshot for the selected event once
each second and publishes a plain snapshot. No interface work.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/runtime/playback_polling.rs` (the reference pattern)
- `src/runtime/mod.rs`
- `src/api.rs` (`fetch_live_metadata_optional`)
- `src/broadcast/registry.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/runtime/broadcast_observation.rs` (new)
- `src/runtime/mod.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`, `src/view_models/**`, `src/app/**`
- `src/playback*` and `src/runtime/playback_polling.rs`
- `src/db.rs`
- `src/http_client.rs`

## Constraints

- Copy the shape of `src/runtime/playback_polling.rs`: a handle, a
  `tokio::sync::watch` sender, and a `oneshot` shutdown.
- Modules under `src/runtime/` import no `gpui`, no `gpui_component`, and no
  module under `src/app`, `src/ui`, or `src/library`.
- The poll interval is one second, as a named constant.
- A transport error becomes a snapshot variant. The actor never panics and
  never stops on an error.
- The actor writes no database rows. Status writes belong to the registry
  service.

## Implementation Steps

1. Add `src/runtime/broadcast_observation.rs` and declare it in
   `src/runtime/mod.rs`.
2. Add `const BROADCAST_POLL_INTERVAL: Duration = Duration::from_secs(1);`.
3. Define `BroadcastObservationSnapshot { at: Instant, outcome }`.
4. Define the outcome enum:
   - `NoEvent` when no event is selected
   - `Live { seq, updated_at, title, destination_count }` from the snapshot body
   - `Empty` when the event exists and holds no payload
   - `Dead` when the relay answers `404`
   - `Error(String)` for a transport failure
5. Add `BroadcastObservationHandle` with `subscribe` and a shutdown on `Drop`,
   like the playback handle.
6. Add `start` that takes the endpoint, an optional event identifier, and a
   cancellation path.
7. Reduce the relay body into display-free facts only. Do not build labels here.
8. Add unit tests on a plain tokio runtime with a stub reader: no event, live,
   empty, dead, and error. Assert that an error does not stop the actor.
9. Add an architecture guard that the new module imports no `gpui` and no
   screen module.

## Acceptance Criteria

- The actor publishes a snapshot each second while it runs.
- Every outcome variant has a test.
- An error tick is followed by a later successful tick in the test.
- The module is GPUI-free and the guard proves it.
- No view model, screen, or database file changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast_observation --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- The relay client cannot be injected as a stub without a trait that this task
  does not define.
- The snapshot body does not carry a destination count without a payload shape
  decision.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0040-async-vm-runtime.md`
- `src/runtime/playback_polling.rs`
- `src/api.rs`

Goal:
- Add `src/runtime/broadcast_observation.rs`, a one-second actor that publishes
  relay snapshots over a `watch` channel.

Constraints:
- Copy the playback polling actor shape.
- No `gpui` import, no screen module import.
- An error becomes a snapshot variant, never a panic and never a stop.
- Publish facts, not display labels.

Do not touch:
- UI, view models, `src/app/**`, database, playback polling

Acceptance criteria:
- Variants: no event, live, empty, dead, error. Each has a test.
- The actor keeps running after an error tick.
- Architecture guard proves the module is GPUI-free.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast_observation --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
