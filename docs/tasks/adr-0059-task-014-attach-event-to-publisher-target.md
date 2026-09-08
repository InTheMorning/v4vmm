# ADR 0059 Task 014: Attach An Event To A Publisher Target

Status: Ready - 2026-09-08. Revised for ADR 0060. Needs `musicindex-live-publisher`
control-surface task 001. Do after 009 and 010.

## Goal

Connect a registered event to a publisher target, so the publisher sends
payloads for the event that the operator selected. This closes the path between
the event registry and the publisher.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/curator-workflow-ui-design-brief.md`
- `docs/architecture/broadcast-chain.md`
- `docs/tasks/adr-0059-task-003-event-registry-service-and-cli.md`
- `docs/tasks/adr-0059-task-010-remote-hosts-over-ssh.md`
- `src/broadcast/registry.rs`, for `BroadcastRegistry`, `CreatedBroadcastEvent`,
  `CheckedBroadcastEvent`, `list_events`, and `forget_event`
- `src/broadcast/tokens.rs`, for `token_path_for_event` and `read_token_file`
- `src/broadcast/transport.rs`
- `src/view_models/show.rs`
- `musicindex-live-publisher`: `docs/tasks/control-surface-task-001-target-management.md`

## Files Likely To Change

- `src/broadcast/publisher_targets.rs` (new)
- `src/broadcast/mod.rs`
- `src/view_models/show.rs`
- `src/ui/shells/show.rs`
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
7. Extend the `Event` section of `ShowPageVm`, following the section
   composition that packet 009 establishes:
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

## Section Order

ADR 0059 orders the sections `Source`, `Publisher`, `Event`, `Stream`.
Packet 015 shipped `Stream` before `Event` existed, so `Stream` currently
renders directly after `Publisher`. Insert `Event` between them. Do not
append it after `Stream`.

## Acceptance Criteria

- An event registered in task 003 can be attached to a publisher target.
- The publisher restarts after a successful attach.
- A publisher without the target commands reports `CommandsUnavailable` and
  changes nothing.
- No token text appears in a command line or in a log.
- The view model carries either the attached target name or a `not attached`
  state, and never an empty string.

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
