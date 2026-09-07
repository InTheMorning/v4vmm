# ADR 0059 Task 014: Attach An Event To A Publisher Target

## Goal

Connect a registered event to a publisher target, so the publisher sends
payloads for the event that the operator selected. This closes the path between
the event registry and the publisher.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `docs/tasks/adr-0059-task-003-event-registry-service-and-cli.md`
- `docs/tasks/adr-0059-task-010-remote-hosts-over-ssh.md`
- `src/broadcast/registry.rs`
- `src/broadcast/transport.rs`
- `src/view_models/broadcast.rs`
- `musicindex-live-publisher`: `docs/tasks/control-surface-task-001-target-management.md`

## Files Likely To Change

- `src/broadcast/publisher_targets.rs` (new)
- `src/broadcast/mod.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/control.rs`
- `src/api.rs`, `src/db.rs` schema
- Any file of `musicindex-live-publisher`

## Constraints

- **This app never writes the publisher configuration file.** Every change runs
  a publisher command through the transport of task 010. This is an ADR 0059
  invariant.
- This task depends on the publisher target commands. Do it after
  `control-surface-task-001` lands in `musicindex-live-publisher`. If those
  commands are absent, report the state as unavailable and stop.
- The token file must exist on the host that runs the publisher. For a remote
  host, the operator copies it. This task does not copy a token over the
  network.
- Never send a token as a command argument. Send the token file path.
- A target name is an input, not a constant. `default` is the first value only.
- Read the target list. Do not cache it across a host change.

## Implementation Steps

1. Add `src/broadcast/publisher_targets.rs`.
2. Add `list_targets(transport, instance)` that runs
   `musicindex-live-publisher target list --json` and parses the result.
3. Add `attach_event(transport, instance, target_name, event_id, token_path)`
   that runs `musicindex-live-publisher target add`.
4. Add `detach_target(transport, instance, target_name)` that runs
   `musicindex-live-publisher target remove`.
5. Map the missing-command case to a `CommandsUnavailable` state, separate from
   a failure. An older publisher does not have these commands.
6. After a successful attach or detach, restart the publisher unit through the
   control service, so the new configuration takes effect.
7. Extend the `Event` section of the view model:
   - show the target name that carries the selected event, or `not attached`
   - add `Attach` and `Detach` actions with typed availability
   - `Attach` is unavailable while the publisher is not reachable
   - `Attach` is unavailable while the event state is `Dead`
   - show a hint when the token file is missing on a remote host
8. Add `v4vmm broadcast targets list --json` and
   `v4vmm broadcast targets attach <event-id> --target <name>` to the CLI.
9. Add tests with a stub runner: list parse, attach, detach, missing command,
   and a publisher that is not reachable.
10. Add a guard: no file outside `src/broadcast/` runs a
    `musicindex-live-publisher` command.

## Acceptance Criteria

- An event registered in task 003 can be attached to a publisher target.
- The publisher restarts after a successful attach.
- A publisher without the target commands reports `CommandsUnavailable` and
  changes nothing.
- No token text appears in a command line or in a log.
- The `Event` section shows the attached target or `not attached`.

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

- The publisher target commands do not exist yet. Stop and report. Do not write
  the publisher configuration file as a substitute.
- The publisher command output shape differs from the task assumption.
- A remote host needs the token file and there is no agreed copy procedure.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/broadcast/registry.rs`, `src/broadcast/transport.rs`

Goal:
- Attach a registered event to a publisher target by running publisher commands
  through the transport.

Constraints:
- Never write the publisher configuration file.
- Never send a token as an argument. Send the file path.
- A missing publisher command is its own state, not a failure.
- Restart the publisher unit after a successful attach.

Do not touch:
- `src/broadcast/control.rs`, API, database schema, the publisher repository

Acceptance criteria:
- Attach, detach, and list work through a stub runner in tests.
- `CommandsUnavailable` changes nothing.
- Guard blocks publisher commands outside `src/broadcast/`.

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
