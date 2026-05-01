# ADR 0024 Task 001: Application Layer Skeleton

## Status

Completed 2026-04-30.

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
- `src/application/events/download.rs`
- `src/application/errors/mod.rs`
- `src/application/errors/command.rs`
- `src/application/ports/mod.rs`
- `src/application/ports/download_manager.rs`
- `src/application/queries/feed.rs`
- `src/presentation/event_bridge.rs`
- `src/presentation/gpui_command_runner.rs`
- `src/presentation/gpui_event_bridge.rs`
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
7. Add the presentation-side `GpuiCommandRunner`, `PresentationEventBridge`,
   and `GpuiEventBridge` skeletons without migrating any workflow.
8. Wire the modules from `src/lib.rs`.
9. Extend `tests/architecture_tests.rs` to fail if `src/application/**` imports
   GPUI or screen modules.

## Acceptance Criteria

- [x] `src/application/` compiles and is GPUI-free.
- [x] `ApplicationServices`, `CommandContext`, `CommandOutcome<T>`,
  `CommandError`, `ApplicationQueryService`, `ApplicationEventBus`, and
  `DownloadManager` exist with the ADR 0024 names.
- [x] `GpuiCommandRunner`, `PresentationEventBridge`, and `GpuiEventBridge` exist
  outside `src/application/`.
- [x] No screen behavior changes.
- [x] Architecture tests cover the new application-layer boundary.

## Result

- Added the dormant `src/application/` skeleton with typed command context,
  command outcome, shared command error, local query service, app-scoped event
  bus, event families, and `DownloadManager` port.
- Added `src/presentation/` skeleton types for GPUI command running and event
  bridging, while keeping GPUI out of `src/application/`.
- Wired the new modules from `src/lib.rs`.
- Extended `tests/architecture_tests.rs` so `src/application/**` cannot import
  GPUI or screen modules.
- No workflow behavior was migrated.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo clippy --lib --tests -- -D warnings -W clippy::pedantic` was attempted;
  it is not green for the existing crate and reports hundreds of pre-existing
  warnings outside this task. New ADR 0024 modules were adjusted for the
  pedantic warnings surfaced in this slice.

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
