# ADR 0052: Library / Index data parity triage

## Status

Implemented - 2026-05-17. Documentation-only triage complete.

## Context

`docs/plans/deferred-architecture-work-index.md` item #2 tracks
Library/Discover data parity for release date, language, explicit state,
description, and related local detail fields. That item was created before
ADR 0048 retired Discover as a top-level screen and before ADR 0049 made
ContentList inspector ownership explicit.

After ADR 0048 and ADR 0049, the live comparison is no longer Library versus
the parked `src/discover/` module. It is:

- Library detail surfaces rendered from local Library projections.
- Index-source detail surfaces rendered through
  `SearchResultsInspector` / `IndexDetailDisplay`.

The parked Discover module remains useful historical context only.

## Decision

Run a documentation-only parity triage before changing runtime code. The
triage compares the visible fields on Library detail surfaces against the
equivalent live Index detail surfaces and routes every gap to exactly one of
these buckets:

- **Loading-shape fix:** the data is already persisted locally, but the read
  model, query, or view-model projection does not surface it. Route to an
  ADR 0024 follow-up plan.
- **Persistence fix:** the data is not stored locally as a source fact. Route
  to source-fact ADR work, either a new ADR or an extension of ADR 0028.
- **Intentional asymmetry:** the field is remote-only, local-only, or
  deliberately omitted by an existing contract. Document the invariant and do
  not create implementation work.

Artist/person identity reconciliation remains owned by ADR 0029. If a parity
gap requires matching remote artists or contributors to durable local person
identity, this triage records it as an open question instead of routing it to
ADR 0024 or source-fact work.

The triage is split into three bounded packets:

- Album / release detail parity.
- Track detail parity.
- Artist and playlist detail parity.

The synthesis step consolidates the three reports into one routing artifact.
Runtime implementation is out of scope for this ADR.

## Invariants

- `src/discover/` is reference-only for this work. It is not the live parity
  target and is not edited.
- Triage reports must include file:line evidence for rendered fields, VM
  sources, persistence, and hydration paths.
- Runtime code, schema, migrations, and ingest behavior do not change during
  this triage.
- Loading-shape fixes route to ADR 0024 follow-up work.
- Persistence fixes route to source-fact ADR work.
- Identity reconciliation routes to ADR 0029 or a future person-identity ADR.
- Intentional asymmetries must name the contract that makes the asymmetry
  deliberate.

## Non-Goals

- No implementation of new Library or Index fields.
- No new search, inspector, or Discover UI behavior.
- No schema migration.
- No artist/person matching or merge policy.
- No deletion or revival of the parked Discover module.

## Alternatives Considered

- **Patch visible gaps directly.** Rejected. The deferred item explicitly
  needs triage because the right route depends on whether a field is missing
  from persistence, projection, or only presentation.
- **Compare against `src/discover/`.** Rejected. ADR 0048 retired Discover as
  the active top-level surface. Live parity now belongs to Index-source
  inspector details.
- **Create a single broad implementation task.** Rejected. Album, track,
  artist, and playlist surfaces have different persistence and identity
  boundaries. Splitting them keeps lower-context work evidence-based.

## Consequences

Positive:

- Downstream work starts with concrete field evidence instead of speculative
  UI fixes.
- ADR 0024, ADR 0028, and ADR 0029 ownership boundaries stay intact.
- The deferred-work index can close item #2 only after every gap has a route.

Negative / risks:

- This ADR produces documentation, not user-visible parity by itself.
- The synthesis may split follow-up work across multiple ADR routes.

## References

- `docs/plans/deferred-architecture-work-index.md`
- `docs/plans/library-discover-parity-triage-plan.md`
- `docs/tasks/library-discover-parity-triage-task-001-album-detail.md`
- `docs/tasks/library-discover-parity-triage-task-002-track-detail.md`
- `docs/tasks/library-discover-parity-triage-task-003-artist-playlist-detail.md`
- ADR 0024 - Command, query, and event application layer
- ADR 0028 - Local identity source-fact persistence
- ADR 0029 - Artist/person identity persistence
- ADR 0048 - ContentList frame breadcrumb search
- ADR 0049 - Inspector source ownership
