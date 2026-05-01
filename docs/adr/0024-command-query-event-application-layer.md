# ADR 0024: Command/Query/Event Application Layer

## Status

Accepted and implemented - 2026-05-01.

Implementation completed through the phase plan in
`docs/plans/adr-0024-application-layer-phase-plan.md` and task packets
`docs/tasks/adr-0024-task-001-*.md` through
`docs/tasks/adr-0024-task-007-*.md`.

## Context

ADR 0023 completed the design-system and view-model foundation for the UI.
The app now has semantic tokens, primitives, composites, shared release
surfaces, stateful `LibraryViewModel` / `SearchViewModel` snapshots, and
architecture tests that keep view-models GPUI-free. That work makes the
presentation boundary explicit, but it does not make the whole application
match the ideal architecture in `docs/architecture/architecture-diagrams.md`.

The target diagrams describe a layered binary:

```text
Presentation Layer       GPUI views, design system, screen event wiring
Application Layer        commands, queries, events, view-model updates
Domain Layer             library, playlist, playback, metadata rules
Infrastructure Layer     SQLite, HTTP clients, RSS, tag I/O, downloader, player
```

The current codebase is partway there. `library.rs`, `search.rs`, and
`app.rs` compose design-system components and read many pure view-model
snapshots, but they still dispatch service calls directly. The screens still
coordinate workflows such as playlist append, feed subscription, track
download, MusicBrainz lookup, feed update, and playback by calling service
modules inline, formatting local status strings, and deciding which snapshots
to reload.

That shape has three problems:

1. The UI remains a workflow coordinator. GPUI screens are thinner than before,
   but a user action still tends to fan out into direct service calls, DB
   refreshes, status string updates, and `cx.notify()` calls in one method.
2. The CLI and UI share service functions directly rather than sharing a typed
   application workflow. This keeps behavior aligned in places, but not through
   a deliberate command/query contract.
3. Event propagation is manual. After each mutation, the caller decides what to
   reload and how to refresh the visible model, which makes cross-view updates
   fragile as the app grows.

ADR 0023 intentionally deferred a broad command/query/event boundary. This ADR
scopes that next step. It moves the app toward the ideal mermaid architecture
without requiring an immediate rewrite of the domain, infrastructure, or screen
directory layout.

## Decision

Introduce a typed application layer that sits between presentation code and the
existing service/domain/infrastructure modules. GPUI should become one thin
presentation adapter over a cohesive, UI-agnostic toolset. The core app should
be structured so another UI could dispatch the same commands, run the same
local read-model queries, consume the same application events, and render its
own presentation without reimplementing workflows.

The application layer will provide three boundaries:

1. `CommandBus` for user-initiated workflows that may mutate state or perform
   side effects.
2. `ApplicationQueryService` for local read-model snapshots consumed by
   view-models and CLI output paths.
3. `ApplicationEventBus` for typed application/domain events emitted after
   command completion and consumed by UI adapters or other application
   subscribers.

The first implementation should create a new `src/application/` module tree:

```text
src/application/
  mod.rs
  application_services.rs
  command_bus.rs
  command_context.rs
  application_query_service.rs
  application_event_bus.rs
  commands/
    mod.rs
    playlist.rs
    feed.rs
    download.rs
    metadata.rs
    playback.rs
    search.rs
  queries/
    mod.rs
    feed.rs
    library.rs
    playlist.rs
    search.rs
    metadata.rs
    playback.rs
  events/
    mod.rs
    download.rs
    library.rs
    playlist.rs
    feed.rs
    metadata.rs
    playback.rs
  ports/
    mod.rs
    download_manager.rs
  errors/
    mod.rs
    command.rs
```

This module tree is the application boundary, not a framework. It should wrap
the existing `*_service` modules first, then allow later ADRs to move purer
business rules into explicit domain modules when that work is justified.
This ADR moves zero existing service modules into new `domain/` or
`infrastructure/` directories.

The `ports/` directory is for application-facing traits where modularity is
already useful. The first required port is `DownloadManager`, so subscription
and download commands do not bind directly to today's download implementation.
A later download manager can replace the current implementation by satisfying
that port without rewriting presentation, command, or view-model code.
Additional ports are introduced only when a phase needs them: MusicBrainz,
RSS, MusicIndex/catalog search, and playback adapters may be wrapped in later
phases. Until then, handlers may call the existing `*_service` modules.

The application boundary is wired through `ApplicationServices`, constructed
at the app root and passed to presentation adapters by `Arc` or an owning
context. `ApplicationServices` holds the shared `CommandBus`,
`ApplicationQueryService`, `ApplicationEventBus`, and port implementations.
It is not a global service locator, string registry, or plugin framework.

### Command boundary

Commands are typed values, not string identifiers or loosely typed maps. A
command must describe one workflow intent and return a typed result.
Command names use imperative verb-object names: `CreatePlaylist`,
`AppendTracksToPlaylist`, `SubscribeFeed`, `DownloadTrack`,
`StageMusicBrainzMatch`, `CheckFeedUpdates`, `PlayTrack`, `PausePlayback`,
`ResumePlayback`, `StopPlayback`, `SeekPlayback`, and
`SetPlaybackVolume`. `StartPlayback` is reserved for a future command that
would initialize the playback subsystem itself, not for playing a track.
Result names mirror the command name where useful:
`CreatePlaylistResult`, `DownloadTrackResult`, `PlayTrackResult`, etc. Phase
plans and task packets must follow this convention for every new command.

Initial command families should cover the high-blast-radius workflows already
called directly from screens:

- playlist creation, deletion, rename, reorder, and append
- feed subscription and unsubscription
- track download, remove, and library membership changes
- MusicBrainz lookup and metadata staging
- feed update checks and update application
- playback transport actions

Command handlers own side effects. If a workflow fetches RSS, calls
MusicBrainz, downloads audio, writes tags, updates SQLite, or calls a player
adapter, that effect belongs inside the command handler or an infrastructure
adapter called by the handler. View-models must not perform those effects, and
GPUI screens must not call the underlying services for migrated workflows.

Command results must be explicit enough for the caller to update user-visible
state without parsing strings. Human-readable status text remains a
presentation/view-model concern.

The command execution contract is deliberately synchronous and GPUI-free:

```rust
type CommandResult<T> = Result<CommandOutcome<T>, CommandError>;

struct CommandOutcome<T> {
    value: T,
    events: Vec<ApplicationEvent>,
}

struct CommandContext {
    operation_id: OperationId,
    cancellation: CancellationToken,
    trace_id: TraceId,
}
```

`CommandBus::execute(command, context)` may block while it talks to SQLite,
RSS, MusicBrainz, the filesystem, tag I/O, `DownloadManager`, or the player
adapter. Therefore GPUI presentation code must not call long-running commands
directly from a render or event handler. The UI must submit those commands
through `GpuiCommandRunner`, a presentation-owned runner that schedules
`CommandBus::execute` on `cx.background_executor()` and then applies the
returned `CommandOutcome` on the GPUI main thread. CLI code may call the same
command bus directly because a blocking one-shot command is acceptable for CLI
execution.

Fast in-memory commands use the same `CommandResult<T>` shape. This keeps one
typed command contract while making the threading decision explicit at the
presentation boundary rather than importing GPUI or an async runtime into
`src/application/`.

Errors are returned through `Result`, not emitted as events. A failed command
may still persist partial state only when the existing workflow already permits
that behavior; the command must then return a typed error that lets the caller
request a query refresh for the affected area. There is no failure event family.

MusicBrainz lookup is a command, not a query. It performs network I/O and may
write staged candidate data, so it belongs in the command boundary and emits a
metadata staging event after persisted changes. Query APIs may read the staged
MusicBrainz status afterward.

Cancellable long-running commands read `operation_id` and cancellation state
from `CommandContext`; the handle does not live in every command payload. The
first implementation may defer full cancellation for a specific workflow, but
only by documenting that workflow as non-cancellable in its task packet and
preserving the current behavior. The command type must still leave room to add
cancellation without changing the UI contract.

`CommandError` is one shared command error channel with family variants, not a
separate unrelated error enum per command. Family variants may wrap
command-specific detail types, for example `CommandError::Playlist(...)`,
`CommandError::Download(...)`, `CommandError::Metadata(...)`, and
`CommandError::Playback(...)`.

### Application query boundary

Application queries are local read-model reads. They return pure Rust snapshots
suitable for view-models, CLI rendering, or tests. The query boundary is not a
general bucket for every read-shaped operation in the program.

Initial query families should cover:

- library tree snapshots
- playlist and playlist-track snapshots
- album/feed/track detail snapshots
- locally cached or staged search result snapshots
- MusicBrainz staging/status snapshots
- feed update status snapshots
- playback state snapshots

`ApplicationQueryService` may call repositories and existing read-only service
functions, but it must not mutate DB state, write files, fetch network data, or
require GPUI types. Network reads belong behind infrastructure ports such as
MusicIndex, MusicBrainz, RSS, or catalog-search clients. If a network read
refreshes local app state, stages facts, or needs progress/cancellation, it is
initiated through a command and its local result is read back through
`ApplicationQueryService`.

Remote-only discovery/search that does not persist or stage local state must
not be hidden inside `ApplicationQueryService` or overloaded onto
`CommandBus`. ADR 0024 does not migrate those workflows unless the operation
refreshes local app state, stages facts, or needs command lifecycle semantics.
A later ADR may introduce a dedicated remote-query/client abstraction. The
important rule is that GPUI must not call MusicIndex, RSS, or MusicBrainz
clients directly for migrated workflows.

### Application event boundary

Commands emit typed events after meaningful state changes. Events describe what
changed, not how a screen should repaint.

Initial event families should include:

- `LibraryChanged`
- `PlaylistChanged`
- `TrackDownloadChanged`
- `FeedSubscriptionChanged`
- `FeedUpdateChanged`
- `MetadataStagingChanged`
- `PlaybackChanged`

View-models consume events by applying deterministic snapshot updates where
the data is already present, or by requesting a read-only query refresh where a
full reload is cheaper and clearer.

The first `ApplicationEventBus` is an in-process typed broadcast publisher for
`ApplicationEvent` batches returned in successful `CommandOutcome`s. It is
app-scoped, not scoped to the screen that dispatched a command. Subscribers may
include Library, Discover, playback, CLI observers, tests, and future UI
adapters. This is what fixes the current cross-view refresh fragility.

`ApplicationEventBus` must not call GPUI APIs directly and must not mutate GPUI
entities from a background thread. The presentation layer owns the bridge from
application events to GPUI:

1. A screen or root app dispatches a command from the GPUI thread.
2. A presentation-owned command runner executes blocking work on
   `cx.background_executor()`.
3. The command returns `CommandResult<T>`.
4. On success, `GpuiCommandRunner` publishes the `ApplicationEvent` batch to
   the app-scoped `ApplicationEventBus`.
5. Each `PresentationEventBridge` subscriber drains relevant events on its UI
   thread, applies them to the relevant view-model entity or triggers a query
   refresh, and then calls its UI notification hook such as `cx.notify()`.
6. On failure, the adapter passes the `CommandError` to the view-model so it can
   render status text and request any needed query refresh.

`PresentationEventBridge` is UI-neutral presentation wiring. The GPUI-specific
implementation should be named `GpuiEventBridge`. Subscribers must not infer
command success from the absence of events; command failure recovery is the
dispatching caller's `Result` responsibility. Do not introduce a cross-thread
actor system, durable event log, or async runtime redesign unless a later ADR
establishes that need.

## Invariants

- No `gpui` or `gpui_component` imports below the presentation layer.
- `src/application/` must expose typed commands, typed queries, typed events,
  and typed errors.
- `ApplicationServices`, `CommandBus`, command handlers, application ports, and
  command contexts that cross the GPUI background boundary must be `Send` and
  `Sync` where the executor requires it.
- Commands own mutation and side effects. Screens and view-models do not call
  service modules for workflows that have migrated to commands.
- Application queries are local-state reads only and return pure data.
- Events are typed. Screens must not infer state changes from formatted status
  strings.
- Command failures travel through typed `Result` errors. Events represent
  state changes only.
- Subscribers must not infer command success from event absence.
- View-models stay GPUI-free and continue to own screen snapshots, local UI
  state, projections, and display-ready status text.
- Infrastructure details such as SQLite, HTTP, filesystem writes, tag I/O,
  audio download, and player control remain behind application, port, service,
  or infrastructure boundaries.
- Presentation adapters, including GPUI, dispatch commands, request local
  snapshots, bridge application events, and render. They do not own workflow,
  persistence, metadata, download, or playback rules.
- Metadata workflows preserve source facts. Do not add hidden inference or
  discard provenance while moving MusicBrainz/feed/tag workflows.
- Each phase plan must name which CLI paths migrate with that workflow and
  which CLI paths remain direct temporarily. A migrated CLI/UI workflow should
  share the same command/query path.
- Each migrated workflow needs focused tests before screen call sites are
  removed.

## Non-goals

- This ADR does not require splitting `library.rs`, `search.rs`, or `app.rs`
  into screen directories before command/query/event boundaries exist.
- This ADR does not redesign the UI, visual theme, tokens, primitives, or
  composites.
- This ADR does not replace GPUI.
- This ADR does not change the SQLite schema by default. Any schema change
  still requires its own migration and test coverage.
- This ADR does not create a generic plugin framework, service locator, or
  reflection-based command registry.
- This ADR moves zero existing services into `domain/` or `infrastructure/`.
  Later ADRs may perform those moves after the application boundary is proven.
- This ADR does not introduce a durable event store.
- This ADR does not introduce a full remote-query abstraction. Remote reads are
  explicit infrastructure ports and become commands when they update local
  state or need command lifecycle semantics.
- This ADR does not require full cancellation support for every long-running
  operation in the first implementation slice. It does require each slice to
  state whether the workflow is cancellable and how cancellation will be added.

## Alternatives considered

### Keep direct screen service calls

Rejected. This preserves the current behavior, but it leaves screens as
workflow coordinators and keeps manual reload/status behavior scattered across
presentation code. It also keeps CLI/UI parity accidental rather than explicit.

### Split large screen files first

Rejected as the next architectural move. File splits would improve navigation,
but they would mostly redistribute the current coupling. The larger benefit
comes from moving service dispatch and reload rules out of GPUI presentation
methods first. Screen splitting can follow once the workflow surface is smaller.

### Add only more view-model methods

Rejected as insufficient. ADR 0023 correctly moved pure state and projection
logic into view-models, and some command-intent values already live there. But
view-models should not become service coordinators. A separate application
layer is needed for side effects and shared CLI/UI workflows.

### Introduce a broad generic bus framework

Rejected. A stringly typed global dispatcher or framework-style bus would hide
control flow and make Rust type checking less useful. The bus should be small,
typed, locally testable, and boring.

### Rewrite into ideal domain/infra directories immediately

Rejected. The target diagrams remain valid, but moving every service,
repository, and adapter at once would create a large blast radius with little
intermediate validation. This ADR favors vertical workflow slices that preserve
behavior while establishing the missing boundary.

## Consequences

### Positive

- GPUI screens become presentation adapters instead of workflow coordinators.
- Mutating workflows become testable without a `Window`, `App`, or GPUI entity
  setup.
- CLI and UI behavior can converge through shared commands and queries.
- Error handling can move from ad-hoc formatted strings to typed `Result`
  failures that view-models render consistently.
- Application-event-driven snapshot refresh gives the app a clear path toward
  the reactive flow shown in the ideal architecture diagrams.
- Download workflows gain a replaceable `DownloadManager` boundary before the
  current implementation becomes harder to swap.
- Later screen splitting becomes safer because service dispatch has a stable
  home outside presentation code.

### Negative

- The repository gains another module boundary and more explicit types.
- During migration, some workflows will temporarily exist in both direct
  service-call form and command form.
- Event ordering and refresh rules need careful tests so the UI does not show
  stale snapshots after command completion.
- A too-generic command bus could obscure straightforward call paths if review
  discipline slips.
- Background command completion needs a GPUI bridge. That bridge must stay
  small and presentation-owned so the application layer remains GPUI-free.
- Remote-only search/discovery remains a boundary decision. This ADR forbids
  hiding network I/O inside local queries or overloading commands with
  stateless remote reads, but a later ADR may choose a richer remote
  query/client abstraction.

### Neutral

- Existing `*_service` modules remain valid implementation dependencies during
  the first phases.
- Existing view-model snapshots remain the presentation read model.
- No public CLI contract changes are implied by this ADR.
- No database migration is implied by this ADR.

## Migration sequence

### Phase 1 - application-layer skeleton

Add `src/application/` with typed command, local query, application event,
port, and error modules. Wire it into the crate without changing screen
behavior. Add tests that prove the layer has no GPUI dependency. Add the GPUI
event-bridge adapter shape, but do not migrate workflows yet.
`ApplicationServices` is constructed at the app root and passed explicitly to
screens, CLI entry points, or test harnesses.

### Phase 2 - playlist vertical slice

Migrate playlist create/delete/rename/reorder/append workflows from direct
screen service calls to commands. Use queries for playlist and playlist-track
snapshots. Emit playlist/library events after mutation. Keep existing UI
behavior and view-model status text. The phase plan must state whether the CLI
playlist path migrates in this phase or remains direct until a named later
task.

After Phase 2, pause for an ADR checkpoint before continuing to
subscription/download, metadata/feed update, or playback. If the
command/context/event boundary is awkward in the playlist slice, revise this
ADR and the remaining task packets before widening the blast radius.

### Phase 3 - subscription and download vertical slice

Migrate feed subscribe/unsubscribe, track download/remove, and library
membership workflows. Preserve existing audio/tagging behavior and source fact
provenance. The screen should dispatch commands and consume typed results or
events, not call subscription services directly. Download commands must expose
an operation id and a cancellation extension point, and must depend on the
`DownloadManager` port rather than a concrete download implementation.

### Phase 4 - metadata and feed update vertical slice

Migrate MusicBrainz lookup/staging and feed update workflows. Keep
MusicBrainz/feed/tag provenance explicit. Avoid metadata inference that is not
already approved by an ADR or plan. MusicBrainz lookup remains a command
because it performs network I/O and stages local metadata.

### Phase 5 - playback vertical slice

Route playback transport actions and playback-state queries through the
application layer while preserving the current `PlaybackOwner<D>` and driver
behavior until a later ADR changes the playback architecture. `PlayTrack`,
`PausePlayback`, `ResumePlayback`, `StopPlayback`, and `SeekPlayback` become
commands; `PlaybackSnapshot` becomes a query. `SetPlaybackVolume` is deferred
until the playback driver boundary has an approved volume operation. Low-level
driver callbacks and process supervision stay inside the existing playback
owner/driver boundary for this ADR.

### Phase 6 - presentation cleanup

After the high-blast-radius workflows are migrated, split large screen files
where it improves comprehension. At this point screen modules should mostly
compose design-system components, bind view-model snapshots, and dispatch typed
commands.

## Test strategy

- Add unit tests for command handlers using in-memory or temp-file DB helpers.
- Add query tests that pin snapshot shapes consumed by view-models.
- Add event tests that verify commands emit the expected event families.
- Extend the existing source-scan architecture tests in
  `tests/architecture_tests.rs`. Follow the ADR 0023 approach: enumerate
  forbidden import/call patterns for migrated screen workflows and fail with
  file/line messages. Use targeted source scans unless a future task proves a
  stronger tool is needed; the expected stronger option is a `syn`-based AST
  test, not ad-hoc shell parsing.
- Keep existing ADR 0023 tests for tokens, primitives, composites, and
  GPUI-free view-models.
- Run `cargo test` for migrated slices and `cargo clippy -- -D warnings` before
  accepting implementation commits.

## Green criteria

This ADR is fulfilled when:

- `src/application/` exists with typed commands, local queries, application
  events, ports, and errors.
- `ApplicationServices`, `CommandContext`, `CommandOutcome<T>`, shared
  `CommandError`, `ApplicationEventBus`, `PresentationEventBridge`,
  `GpuiCommandRunner`, and `GpuiEventBridge` are named consistently with this
  ADR.
- `src/application/ports/download_manager.rs` exists and download/subscription
  commands depend on that port rather than a concrete downloader.
- `ApplicationEventBus` broadcasts command event batches to app-level
  subscribers; it is not a per-screen local dispatcher.
- Migrated UI workflows dispatch commands rather than calling service modules
  directly.
- Each phase plan records the CLI migration decision for its workflow. When a
  CLI path is marked migrated, it uses the same command/query path as the UI.
- Library, Discover, and playback local snapshots are loaded through
  `ApplicationQueryService` APIs for migrated workflows.
- Commands emit typed events for playlist, library, feed, metadata, download,
  and playback changes.
- View-models consume application-event and local-query output without
  importing GPUI.
- `tests/architecture_tests.rs` prevents GPUI imports below presentation and
  prevents migrated screen workflows from regressing to direct service
  dispatch.
- Existing behavior is preserved across playlist, subscription/download,
  metadata/feed-update, and playback vertical slices.
- `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt -- --check` pass.

## Follow-up work

- Decide whether staged metadata needs a durable read model or should remain
  transient GPUI-free view-model state.
- Define a volume operation on the playback driver boundary before adding
  `SetPlaybackVolume`.
- Revisit playback owner/driver process supervision in a later playback
  architecture ADR.
- Decide whether remote discovery/search and remote inspector reads need their
  own remote-query boundary. This ADR intentionally keeps remote-only reads out
  of `ApplicationQueryService`.
- Revisit explicit `domain/` and `infrastructure/` directories after the
  command/query/event boundary is proven through at least two migrated
  workflows.
- Replace source-scan architecture tests with a `syn`-based AST test only if
  the current scans become too brittle.

## References

- ADR 0015 - Non-UI Service Boundaries.
- ADR 0022 - UI-Agnostic Core Extraction.
- ADR 0023 - Design System and View-Model Architecture.
- `docs/architecture/architecture-diagrams.md` - current and ideal mermaid
  diagrams.
- `docs/plans/adr-0023-design-system-migration.md` - completed ADR 0023 work
  and deferred command/query/event architecture.
- `docs/plans/adr-0023-finalization-plan.md` - final ADR 0023 implementation
  sequence and deferred screen split.
