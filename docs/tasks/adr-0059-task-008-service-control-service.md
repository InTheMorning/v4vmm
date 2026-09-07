# ADR 0059 Task 008: Publisher Service Control

## Goal

Add a GPUI-free service that reads and changes the state of the publisher unit
and the producer unit with `systemctl --user`. No UI.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/audio_format.rs` (the existing subprocess pattern)
- `src/broadcast/mod.rs`
- `musicindex-live-publisher`: `systemd/musicindex-live-publisher@.service`
- `musicindex-live-publisher`: `systemd/mixxx-now-playing.service`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/broadcast/control.rs` (new)
- `src/broadcast/mod.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`, `src/view_models/**`, `src/app/**`
- `src/api.rs`, `src/db.rs`
- `src/broadcast/registry.rs`

## Constraints

- The service is GPUI-free and blocks. A later task calls it from an actor.
- Read state with one call:
  `systemctl --user show <unit> --property=LoadState,ActiveState,SubState,Result`.
  Parse the key and value pairs. Do not parse the human status text.
- Map the result to a typed state:
  - `LoadState=not-found` becomes `NotInstalled`
  - `ActiveState=failed` becomes `Failed`, and `Result` carries the reason
  - `ActiveState=active` becomes `Active`
  - `ActiveState=inactive` becomes `Inactive`
  - anything else becomes `Unknown`
- **A failed unit needs `reset-failed` before `start` does anything.** The unit
  file sets `StartLimitBurst=5`, so a wrong token drives the unit to `failed`
  and holds it there. Expose `reset` as its own operation.
- Unit names are values, not constants in shared logic. The publisher unit is
  an instance unit, so the instance name is an input.
- Never place a token, a password, or a file content in a command line.
- Tests must not call `systemctl`. Parse recorded output strings instead.

## Implementation Steps

1. Add `src/broadcast/control.rs`.
2. Define `ServiceState` with the six variants above, and a reason field on
   `Failed`.
3. Define `UnitRef { unit: String }` and build the publisher unit name from an
   instance name, in the form `musicindex-live-publisher@<instance>.service`.
4. Add a `CommandRunner` trait with one method that runs a program with
   arguments and returns the exit status, stdout, and stderr. Add a real
   implementation with `std::process::Command`. Tests use a stub.
5. Add `show(unit)`, `start(unit)`, `stop(unit)`, `restart(unit)`,
   `reset(unit)`, and `daemon_reload()`.
6. Add `logs(unit, lines)` that runs
   `journalctl --user -u <unit> -n <lines> --no-pager` and returns the text.
7. Add unit tests with recorded `systemctl show` output for each state, plus a
   missing unit, and a `journalctl` output sample.
8. Add a test that `start` on a `Failed` state is reported as not useful, so the
   caller reaches for `reset` first.
9. Add an architecture guard: `src/broadcast/control.rs` imports no `gpui`, and
   no file outside `src/broadcast/` runs `systemctl`.

## Acceptance Criteria

- Every state maps from recorded output, including `not-found`.
- A failed unit reports its `Result` value as the reason.
- The log read returns text and never panics on an empty journal.
- No test runs a real `systemctl`.
- The guard blocks `systemctl` calls outside the module.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast::control --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- `systemctl show` does not carry a property that a state needs.
- The producer unit and the publisher unit need different state rules.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/audio_format.rs` for the subprocess pattern
- the two unit files in `musicindex-live-publisher/systemd/`

Goal:
- Add `src/broadcast/control.rs`, a GPUI-free `systemctl --user` wrapper with a
  typed state and a log read.

Constraints:
- Parse `systemctl show --property=...`, never the human status text.
- `Failed` is its own state and needs `reset-failed` before a start works.
- Unit names are inputs, not constants in shared logic.
- No secret in a command line. No real `systemctl` in tests; use a stub runner.

Do not touch:
- UI, view models, `src/app/**`, API, database, the registry service

Acceptance criteria:
- Six states map from recorded output, `not-found` included.
- Failed carries the `Result` reason.
- Guard blocks `systemctl` outside `src/broadcast/`.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast::control --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
