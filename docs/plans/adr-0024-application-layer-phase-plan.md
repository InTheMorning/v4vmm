# ADR 0024 Application Layer Phase Plan

## Goal

Implement ADR 0024 incrementally: introduce a GPUI-free application layer with
typed commands, local read-model queries, application events, root wiring, and
replaceable ports, then migrate high-blast-radius workflows one vertical slice
at a time.

The end state is not "GPUI removed." The end state is that GPUI is a thin
presentation adapter over a cohesive application toolset that another UI could
reuse without reimplementing playlist, subscription, download, metadata, or
playback workflows.

## Non-goals

- Do not move existing services into `domain/` or `infrastructure/` directories
  in this ADR.
- Do not introduce a durable event store, actor system, or async runtime.
- Do not hide remote network reads inside `ApplicationQueryService`.
- Do not split `library.rs`, `search.rs`, or `app.rs` before service dispatch is
  migrated out of the relevant workflow.
- Do not redesign UI visuals, tokens, primitives, composites, or theme.
- Do not change SQLite schema unless a later task explicitly requires a
  migration and its own review.

## Current State

- ADR 0023 finalized design tokens, primitives, composites, shared release
  surfaces, and GPUI-free view-model snapshots.
- `LibraryViewModel` and `SearchViewModel` own pure snapshots and many local UI
  transitions.
- `library.rs`, `search.rs`, and `app.rs` still call service modules directly
  for playlist, subscription/download, metadata/feed update, and playback
  workflows.
- Cross-view refresh is still manual: the caller decides what to reload and when
  to call `cx.notify()`.
- Architecture tests in `tests/architecture_tests.rs` already enforce ADR 0023
  source-scan boundaries.

## Target State

- `src/application/` exists with:
  - `ApplicationServices`
  - `CommandBus`
  - `CommandContext`
  - `CommandOutcome<T>`
  - shared `CommandError`
  - `ApplicationQueryService`
  - `ApplicationEvent`
  - `ApplicationEventBus`
  - application ports, starting with `DownloadManager`
- `src/application/events/download.rs` exists for download state-change events.
- `src/application/queries/feed.rs` exists for feed update local snapshots.
- GPUI code uses `GpuiCommandRunner` and `GpuiEventBridge` instead of invoking
  long-running workflows inline.
- Application events are app-scoped broadcasts, not local per-screen callbacks.
- Migrated workflows use commands for mutation/side effects and
  `ApplicationQueryService` for local snapshots.
- Remote-only discovery/search remains outside this ADR unless it stages or
  refreshes local state.
- A checkpoint after the playlist slice confirms the boundary before download,
  metadata, and playback migration widens the blast radius.

## Assumptions

- Blocking command execution is acceptable inside the application layer as long
  as presentation code schedules long-running commands off the GPUI event/render
  path.
- `ApplicationServices` can be passed explicitly to GPUI screens, CLI entry
  points, and tests by `Arc` or an owning context.
- Existing `*_service` modules remain the implementation substrate for early
  phases.
- Command handlers and ports that cross GPUI background execution boundaries
  need `Send + Sync` where required by the executor.
- The first source-scan architecture gates can follow the ADR 0023 style; a
  `syn`-based AST check is the preferred stronger option if scans become too
  brittle.

## Affected Modules

- `src/application/**` (new)
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `src/cli.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/playlist_service.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/metadata_service.rs`
- `src/playback.rs`
- `src/playback_owner.rs`
- `tests/architecture_tests.rs`
- Existing test helpers under `tests/common/`

## Proposed Sequence

1. Completed `adr-0024-task-001-application-skeleton`: add application modules, root
   wiring types, command context/outcome/error/event/query shapes, the GPUI
   runner/bridge skeleton, and architecture tests. Do not migrate workflows.
2. Completed `adr-0024-task-002-playlist-vertical-slice`: migrate playlist
   create/delete/rename/reorder/remove and existing-local-track append through
   commands, local queries, and app events. Subscription/download-backed
   playlist append paths remain in Task 004.
3. Completed `adr-0024-task-003-phase-2-checkpoint`: playlist commands,
   queries, and architecture gates validated the boundary, but Task 004 must
   wire app-level event consumption before widening the blast radius.
4. Completed `adr-0024-task-004-subscription-download-slice`: app-level
   application-event consumption, feed unsubscribe, track remove, local
   library-membership, feed subscribe, track subscribe/download, and
   subscribe-then-append workflows route through commands and the
   `DownloadManager` port. No matching CLI subscription/download commands exist
   to migrate in this slice.
5. `adr-0024-task-005-metadata-feed-update-slice`: migrate MusicBrainz staging
   and feed update workflows; preserve source facts and metadata provenance.
6. `adr-0024-task-006-playback-slice`: route playback transport commands and
   playback snapshots through the application layer while preserving the
   existing playback owner/driver boundary.
7. `adr-0024-task-007-presentation-cleanup`: after migrated workflows leave the
   screens, split or simplify presentation modules only where it improves
   comprehension.

## Schema/API Implications

No schema changes are expected. Public CLI behavior should remain compatible.
Each phase must explicitly record whether its related CLI paths migrate to the
application layer or remain direct until a named later task.

## Risk Areas

- Accidentally introducing GPUI imports into `src/application/`.
- Making `ApplicationEventBus` a per-screen callback rather than app-scoped
  broadcast.
- Running blocking command handlers directly from GPUI event handlers.
- Letting `ApplicationQueryService` become a remote network client.
- Losing playlist ordering/dedup behavior while moving calls out of screens.
- Hiding metadata inference inside migrated MusicBrainz/feed workflows.
- Building a generic service locator instead of explicit `ApplicationServices`.
- Creating non-`Send` command handlers or ports that cannot run on the GPUI
  background executor.

## Test Strategy

- Run `cargo fmt -- --check` and `cargo check` after each implementation task.
- Run focused tests for affected command/query/event modules.
- Run existing service tests when a command wraps a service workflow.
- Extend and run `cargo test --test architecture_tests` for boundary gates.
- Run `cargo clippy --lib --tests -- -D warnings` before merging a phase.
- Run full `cargo test` before marking ADR 0024 complete.

## Rollback Strategy

Each task must be revertible independently. Phase 1 should add dormant
application-layer types and tests before any screen behavior changes. The
playlist slice must land before subscription/download work so the architecture
can be revised after the checkpoint without carrying partially migrated
download or metadata code.

## Pause / Revise Criteria

Pause after Phase 2 if any of these are true:

- `CommandContext` does not carry enough operation/cancellation/tracing data.
- `ApplicationEventBus` does not reliably update multiple views.
- `ApplicationServices` feels like a service locator instead of explicit root
  wiring.
- `ApplicationQueryService` needs remote network behavior to satisfy a migrated
  workflow.
- Architecture tests cannot enforce the intended boundary without excessive
  false positives.

If triggered, revise ADR 0024 and the remaining task packets before Phase 3.
