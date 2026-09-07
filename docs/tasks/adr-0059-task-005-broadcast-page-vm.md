# ADR 0059 Task 005: Broadcast Page View Model

## Goal

Define the GPUI-free `BroadcastPageVm` display contract for the three sections:
`Source`, `Publisher`, and `Event`. No shell, no adapter.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/view_models/queue_now_playing.rs` (display-contract precedent)
- `src/view_models/workspace/chrome.rs` (`FrameChromeButtonDisplay`)
- `src/view_models/mod.rs`
- `src/runtime/broadcast_observation.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/broadcast.rs` (new)
- `src/view_models/mod.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`, `src/app/**`
- `src/db.rs`, `src/api.rs`
- `src/broadcast/**`
- `src/view_models/queue_now_playing.rs`

## Constraints

- Zero `gpui` imports.
- Display-ready fields only. No `Duration`, no database row, no service handle,
  and no `anyhow::Error` in the public surface.
- Every action carries a typed state and an accessibility label. Reuse
  `FrameChromeButtonDisplay` where it fits.
- Every section has an explicit empty state. `Publisher` needs a
  "not installed or not reachable" state, which is an empty state and not an
  error string.
- State enums, not booleans, for service state and event state. A failed unit is
  its own variant, separate from stopped.
- Never hold token text. The event section shows the token path only.
- Documented public types.

## Implementation Steps

1. Add `src/view_models/broadcast.rs` and declare it in `src/view_models/mod.rs`.
2. Define `SourceSectionDisplay`:
   - source name, source kind label, host label
   - `SourceState { Unknown, Idle, Playing, Paused, NotReachable }`
   - current track title and artist labels, both optional
   - a readiness label slot for task 012, empty until then
3. Define `PublisherSectionDisplay`:
   - unit name labels for the publisher unit and the producer unit
   - `ServiceState { Active, Inactive, Failed, NotInstalled, NotReachable, Unknown }`
   - start, stop, and reset actions with typed availability
   - a logs action
   - a failure reason label, optional
4. Define `EventSectionDisplay`:
   - event label, event identifier, endpoint, token path
   - `EventState { None, Unknown, Live, Dead }`
   - create, resume, and forget actions with typed availability
   - a listener truth line built from the observation outcome
5. Define `BroadcastPageVm { source, publisher, event, empty_label }` and a
   builder.
6. Add a projector that takes plain inputs and returns the view model. The
   inputs are the observation outcome, the service states, the selected event
   row, and the source facts.
7. Rules the projector must hold:
   - `Start` is unavailable while the state is `Active`
   - `Start` is unavailable while the state is `Failed`, and `Reset` is
     available instead
   - `Resume` is unavailable while the event state is `Dead`
   - `Forget` stays available for a dead event
8. Add unit tests for: no event, live event, dead event, failed unit, publisher
   not installed, source paused, and source not reachable.
9. Add an architecture guard: the module is GPUI-free, holds the three section
   types, and contains no field named `token` other than `token_path`.

## Acceptance Criteria

- The module compiles and documents its public types.
- No `gpui` import and no service or database type in the public surface.
- Service state and event state are enums, and `Failed` is distinct from
  `Inactive`.
- The availability rules above are covered by tests.
- The guard blocks a token field.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- A section needs a fact that the observation outcome does not carry.
- `FrameChromeButtonDisplay` does not fit an action and a new shared type is
  needed. Say so before you add a local copy.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/view_models/queue_now_playing.rs`
- `src/view_models/workspace/chrome.rs`
- `src/runtime/broadcast_observation.rs`

Goal:
- Add `src/view_models/broadcast.rs` with `BroadcastPageVm` and the three
  section display types.

Constraints:
- GPUI-free. Display-ready fields only.
- Enums for service state and event state. `Failed` is its own variant.
- Every action has typed availability and an accessibility label.
- Empty state for a publisher that is missing or not reachable.
- Token path only, never token text.

Do not touch:
- `src/ui/**`, `src/app/**`, `src/db.rs`, `src/api.rs`, `src/broadcast/**`

Acceptance criteria:
- Availability rules for start, reset, resume, and forget are tested.
- Architecture guard proves the module is GPUI-free and holds no token field.

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
4. deviations from task
5. unresolved concerns
