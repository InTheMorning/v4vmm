# ADR 0059 Task 010: Remote Hosts Over SSH

## Goal

Let the control service and the source reader reach a host that runs the
publisher on another machine. Use `ssh`. Add reachability as its own state.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/broadcast/control.rs`
- `src/config.rs`
- `src/view_models/broadcast.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/broadcast/transport.rs` (new)
- `src/broadcast/control.rs`
- `src/config.rs`
- `src/view_models/broadcast.rs`
- `src/ui/shells/broadcast.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/api.rs`, `src/db.rs`
- `src/broadcast/registry.rs`
- `src/runtime/broadcast_observation.rs`

## Constraints

- Model hosts as a list from the first commit, as ADR 0059 requires. The
  interface shows one selection. Adding a second host later must change no data
  shape.
- A transport is `Local` or `Ssh { destination }`. The control service takes a
  transport and builds the same command for both.
- **Reachability is its own state.** A host that does not answer is not a failed
  unit and not a missing publisher.
- Never pass a secret on a command line. `ssh` uses the operator keys and the
  agent. Do not add a password option and do not add a key path option in this
  task.
- Do not build a shell string. Pass the program and the arguments as a list, so
  a host name cannot inject a command.
- The relay read needs no transport. It already works from anywhere.

## Implementation Steps

1. Add `src/broadcast/transport.rs` with a `Transport` enum and a `run` method
   that wraps the command runner. For `Ssh`, the program is `ssh` and the
   arguments start with the destination, followed by the original program and
   its arguments.
2. Add `-o BatchMode=yes` and a connect timeout to the `ssh` arguments so an
   unreachable host fails fast instead of asking for a password.
3. Change the control service functions to take a transport.
4. Add `Reachable` and `NotReachable` results. Map an `ssh` exit that fails to
   connect to `NotReachable`, and separate it from a `systemctl` failure.
5. Add a `[broadcast]` config section with a host list. Each entry has a name, a
   transport, and an instance name. A missing section means one local host.
   An older `config.toml` must keep loading.
6. Add a drop-file read through the transport, with `cat` on the drop file path.
   A missing file means no track plays.
7. Extend the view model with the host name and the reachability state, and
   render the host in the `Source` section.
8. Add unit tests with a stub runner: local command shape, ssh command shape,
   argument order, unreachable host, and a host name that holds a space or a
   semicolon.
9. Add a guard: no file builds an `ssh` command outside `src/broadcast/`.

## Acceptance Criteria

- The same control operations work for a local host and an ssh host.
- An unreachable host shows `NotReachable`, not `Failed` and not
  `NotInstalled`.
- A host name with shell characters cannot inject a command.
- A `config.toml` written before this task still loads.
- The host list holds more than one entry without a data shape change.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test broadcast --lib --quiet`
- `cargo test config --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- The exit code of `ssh` cannot separate a connect failure from a remote command
  failure.
- The config format cannot hold a host list without a change that breaks an
  older file.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `src/broadcast/control.rs`, `src/config.rs`

Goal:
- Add an `ssh` transport for the control service and the drop-file read, plus a
  host list in config.

Constraints:
- Hosts are a list from the start. The interface selects one.
- Reachability is its own state, separate from failed and not installed.
- Pass program and arguments as a list. Never build a shell string.
- No password option and no secret on a command line. Use `BatchMode=yes`.

Do not touch:
- API, database, the registry service, the observation actor

Acceptance criteria:
- Local and ssh use the same operations.
- Unreachable maps to its own state.
- A host name with shell characters is safe.
- An older `config.toml` still loads.

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
