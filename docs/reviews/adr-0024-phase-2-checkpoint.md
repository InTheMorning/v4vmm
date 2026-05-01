# ADR 0024 Phase 2 Checkpoint

## Reviewed Artifact

- `docs/adr/0024-command-query-event-application-layer.md`
- `docs/plans/adr-0024-application-layer-phase-plan.md`
- `docs/tasks/adr-0024-task-001-application-skeleton.md`
- `docs/tasks/adr-0024-task-002-playlist-vertical-slice.md`
- Staged playlist-slice implementation diff

## Status

- Result: Pass with required Task 004 adjustment.
- Reviewer: Codex
- Date: 2026-04-30

## Findings

- `CommandContext` is sufficient for the playlist slice and already carries the
  operation id, cancellation token, and trace id needed by later long-running
  commands.
- `ApplicationServices` stayed explicit root wiring. It is passed into screens
  as an `Arc` and does not provide dynamic string lookup or service-location
  behavior.
- `ApplicationQueryService` stayed local-only. Playlist snapshots read SQLite
  through the existing playlist service and did not hide remote discovery or
  MusicBrainz reads.
- `CommandBus` and playlist command handlers remained GPUI-free. Screen code
  dispatches migrated playlist mutations through `GpuiCommandRunner`.
- Architecture source-scan tests were useful and low-noise for this slice. The
  new playlist gate prevents `library.rs`, `search.rs`, and `app.rs` from
  reintroducing direct `playlist_service` calls.
- `ApplicationEventBus` broadcasts emitted event batches, but no app-level GPUI
  subscriber is wired yet. Playlist views still refresh from local command
  success callbacks and existing screen events. This does not invalidate the
  boundary, but Task 004 must wire the presentation event bridge before widening
  to subscription/download workflows.

## Required Revisions

- Task 004 must begin by wiring `GpuiEventBridge` or equivalent app-level
  subscribers so application events can refresh affected screens from a shared
  broadcast path.
- Task 004 owns the remaining `library_service::subscribe_then_append_to_playlist`
  callers because those workflows combine subscription/download and playlist
  append behavior.
- Keep CLI playlist migration deferred unless a later task explicitly pulls CLI
  paths through the application layer.

## Optional Improvements

- Add an event-bus subscriber integration test once the GPUI bridge is wired.
- Consider replacing source scans with a `syn`-based architecture test only if
  the scan rules become brittle in later phases.

## Decision

Proceed to Task 004 after applying the task-packet adjustment above. ADR 0024
does not need a semantic rewrite before Phase 3.
