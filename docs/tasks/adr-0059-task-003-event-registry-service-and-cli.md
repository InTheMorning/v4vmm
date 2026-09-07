# ADR 0059 Task 003: Event Registry Service And CLI

## Goal

Add a GPUI-free registry service for create, list, forget, and a liveness check.
Expose it through CLI commands, as ADR 0017 requires. No UI.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0017-cli-debug-contracts.md`
- `docs/tasks/adr-0059-task-002-event-registry-schema.md`
- `src/api.rs` (`create_live_item`, `fetch_live_metadata_optional`)
- `src/cli.rs`
- `src/broadcast/tokens.rs`
- `src/db.rs`

## Files Likely To Change

- `src/broadcast/registry.rs` (new)
- `src/broadcast/mod.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`, `src/view_models/**`, `src/app/**`
- `src/http_client.rs`
- `src/playback*`
- The `broadcast_events` schema from task 002

## Constraints

- The service is GPUI-free. No `gpui` import.
- Build every HTTP client through `api::Client`, which uses
  `src/http_client.rs`. ADR 0058 forbids a second construction site.
- `create` writes the token file first, then inserts the row. If the insert
  fails, remove the token file.
- A `404` from the metadata read means a dead event. Any other error is a
  transport failure, and must not mark the event dead.
- `forget` removes the row and the token file. It sends nothing to the relay.
- **The app never replaces a dead event on its own.** Report the state and stop.
- Never print a token. Never put a token in an error message.

## Implementation Steps

1. Add `src/broadcast/registry.rs` with a `BroadcastRegistry` type that holds a
   database handle and an endpoint.
2. Add `create_event(label)`:
   - call `api::Client::create_live_item`
   - write the token to a file under the token directory, named by event
     identifier
   - insert the row with `last_status = "unknown"`
   - return the event row and the token path, never the token text
3. Add `list_events()` that returns the stored rows.
4. Add `forget_event(event_id)` that deletes the row and the token file.
5. Add `check_event(event_id)`:
   - call `fetch_live_metadata_optional`
   - `Some(_)` or a `200` empty snapshot means `live`
   - a `404` means `dead`
   - a transport error leaves the stored status unchanged and returns the error
   - write `last_checked_at` and `last_status` on a definite answer
6. Add CLI commands under a `broadcast` section:
   - `v4vmm broadcast events list --json`
   - `v4vmm broadcast events create --json [--label <text>]`
   - `v4vmm broadcast events forget <event-id>`
   - `v4vmm broadcast events check <event-id> --json`
7. Print the token path in `create` output. Do not print the token.
8. Update the CLI usage text.
9. Document the commands in `docs/runbooks/workflows.md`, in a new
   `Manage Broadcast Events` section.
10. Add unit tests with a stub relay response for create, dead, live, and
    transport failure.
11. Add an architecture guard: `src/broadcast/**` imports no `gpui`, and
    constructs no `reqwest` client.

## Acceptance Criteria

- `create` writes a `0600` token file and one row.
- A failed insert leaves no token file behind.
- `check` maps `404` to `dead` and leaves the status unchanged on a transport
  error.
- `forget` removes the row and the file.
- No command prints token text.
- The runbook documents the four commands.
- The guard blocks `gpui` and direct client construction under
  `src/broadcast/`.

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

- `fetch_live_metadata_optional` cannot separate `404` from a transport error.
- The CLI structure cannot host a two-word command section without a parser
  change that this task does not cover.
- A stub relay needs a new test dependency.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/adr/0017-cli-debug-contracts.md`
- `src/api.rs`, `src/cli.rs`, `src/broadcast/tokens.rs`, `src/db.rs`

Goal:
- A GPUI-free registry service for create, list, forget, and check, plus CLI
  commands.

Constraints:
- No `gpui` import and no direct `reqwest` client. Use `api::Client`.
- Write the token file before the row; clean up the file if the insert fails.
- `404` means dead. A transport error changes no stored status.
- Never replace a dead event automatically. Never print a token.

Do not touch:
- UI, view models, `src/app/**`, playback, `src/http_client.rs`

Acceptance criteria:
- Create, list, forget, and check work and are covered by tests.
- No token text reaches stdout or an error message.
- Architecture guard blocks `gpui` and client construction in
  `src/broadcast/`.

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
