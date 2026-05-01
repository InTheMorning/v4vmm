# ADR 0024 Task 001: Application Layer Skeleton

## Status

Planned.

## Task Goal

Add the dormant ADR 0024 application-layer skeleton without migrating any
workflow behavior.

## Files To Inspect

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `src/lib.rs`
- `src/app.rs`
- `src/cli.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/application/mod.rs`
- `src/application/application_services.rs`
- `src/application/command_bus.rs`
- `src/application/command_context.rs`
- `src/application/application_query_service.rs`
- `src/application/application_event_bus.rs`
- `src/application/events/mod.rs`
- `src/application/errors/mod.rs`
- `src/application/errors/command.rs`
- `src/application/ports/mod.rs`
- `src/application/ports/download_manager.rs`
- `src/lib.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- `src/playlist_service.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/metadata_service.rs`
- `src/playback.rs`
- Database migrations.

## Constraints

- Do not migrate workflow behavior in this task.
- Do not add GPUI imports under `src/application/`.
- Do not introduce a service locator or string command registry.
- Keep command execution synchronous and GPUI-free.
- Types that cross the GPUI background boundary must be `Send + Sync` where the
  executor requires it.

## Implementation Steps

1. Add `src/application/` module tree named in ADR 0024.
2. Define `ApplicationServices` with explicit fields for command bus, query
   service, event bus, and ports.
3. Define `CommandContext`, `CommandOutcome<T>`, `CommandResult<T>`, and shared
   `CommandError`.
4. Define an app-scoped `ApplicationEventBus` API that can broadcast typed event
   batches to subscribers without depending on GPUI.
5. Add empty command/query/event family modules needed by later tasks.
6. Add `DownloadManager` as an application-facing port trait.
7. Wire the module from `src/lib.rs`.
8. Extend `tests/architecture_tests.rs` to fail if `src/application/**` imports
   GPUI or screen modules.

## Acceptance Criteria

- `src/application/` compiles and is GPUI-free.
- `ApplicationServices`, `CommandContext`, `CommandOutcome<T>`,
  `CommandError`, `ApplicationQueryService`, `ApplicationEventBus`, and
  `DownloadManager` exist with the ADR 0024 names.
- No screen behavior changes.
- Architecture tests cover the new application-layer boundary.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The skeleton requires a new async runtime.
- `ApplicationServices` needs hidden global state.
- A GPUI type appears necessary under `src/application/`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `src/lib.rs`
- `tests/architecture_tests.rs`

Goal:
- Add the dormant ADR 0024 application-layer skeleton and architecture gates.

Constraints:
- Do not migrate workflow behavior.
- No GPUI imports under `src/application/`.
- Keep command execution synchronous and typed.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- `src/playlist_service.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/metadata_service.rs`
- `src/playback.rs`
- Database migrations.

Acceptance criteria:
- Named application-layer types exist and compile.
- Architecture tests enforce the GPUI-free application boundary.
- Existing behavior is unchanged.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
