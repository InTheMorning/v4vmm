# Library / Discover (Index) Data Parity Triage Plan

## Status

Completed - 2026-05-17. Reports and synthesis landed.

Governing ADR: `docs/adr/0052-library-index-data-parity-triage.md`.
Review checklist:
`docs/reviews/library-discover-parity-triage-review-checklist.md`.
Synthesis:
`docs/reviews/library-discover-parity-triage-synthesis.md`.
Follow-up routing:
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md` and
`docs/adr/0053-local-detail-source-fact-parity.md`.

## Goal

Resolve deferred-architecture-work-index item #2: *Library/Discover data parity
for release date, language, explicit state, description, and related local
detail fields.*

The triage produces, for every visible-field gap between the Library detail
surfaces and the equivalent Index-source detail surfaces (rendered via
`SearchResultsInspector` + `IndexDetailDisplay` after ADR 0048/0049), a routing
decision:

- **Loading-shape fix** — data is persisted but the read model / query path
  does not surface it. Route: ADR 0024 follow-up plan + bounded slice.
- **Persistence fix** — data is not stored locally at all. Route: source-fact
  ADR work (new ADR or extension of ADR 0028).
- **Intentional asymmetry** — gap is by design (e.g., remote-only field,
  contract boundary). Route: document in the relevant ADR's invariants;
  no code change.

## Context

The deferred-work index entry was written 2026-05-08, before ADR 0048
(2026-05-16) unified the Discover top-level tab into the ContentList frame's
search inspector. "Discover" in the entry now refers to the **Index-source
detail views** owned by `IndexDetailDisplay` (ADR 0049), not the parked
`src/discover/` module (see `docs/notes/2026-05-discover-module-parked.md`).

The parity comparison is therefore:

- **Library side:** `src/ui/shells/library/{detail,feed_detail,track_detail,
  playlist_detail}.rs` driven by `src/view_models/library.rs` projections of
  local feed/release/track/playlist views.
- **Index side:** `src/view_models/search_results/index_detail.rs` and
  `SearchResultsInspector` rendering of MusicIndex-source rows for the same
  entity kinds.

Post-ADR-0028 already closed contributor identity visibility. Remaining named
gaps from the index entry:

- release date
- language
- explicit state
- description
- related local detail fields (catch-all — the triage enumerates them
  per-surface)

## Non-Goals

- No runtime code changes during triage. Output is documentation.
- No new schema, migration, or ingest behavior.
- No reopening of the `src/discover/` parked-module status.
- No artist/person identity reconciliation work (routed to ADR 0029).
- No remote-only Discover read thinning (that is index item #3, separate
  follow-up).

## Triage shape

Three parallel research tasks, one per detail surface family. Each subagent
produces a triage report file at
`docs/reviews/library-discover-parity-triage-{family}.md` with this schema:

```
## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |

## Gap analysis (per missing/divergent field)

### Field: <name>
- Library renderer: <file:line>
- Index renderer: <file:line or "not rendered">
- Library VM source: <type::field, file:line>
- Index VM source: <type::field, file:line>
- Local persistence today: <table.column or "not persisted">
- Hydration path: <where it would arrive if persisted>
- Routing: loading-shape | persistence | intentional asymmetry
- Rationale: <one paragraph>

## Open questions
- <items needing user decision before routing>
```

The same-shape outputs let the synthesis step consolidate routing without
re-reading source.

## Proposed Sequence

1. **Task 001 — Album / release detail parity.** Compare Library album-detail
   inspector (`src/ui/shells/library/{detail,feed_detail}.rs`) against the
   Index album/release detail rendered by `IndexDetailDisplay`. Subagent.
2. **Task 002 — Track detail parity.** Compare Library track-detail surfaces
   (`src/ui/shells/library/track_detail*.rs` + metadata grid) against Index
   track detail. Subagent.
3. **Task 003 — Artist + Playlist detail parity.** Compare the artist and
   playlist Library detail surfaces against their Index equivalents (or
   document absence). Subagent.
4. **Synthesis (main thread).** Consolidate the three triage reports into a
   single routing recommendation. Output is either:
   - A new ADR draft (if persistence fixes dominate), or
   - An ADR 0024 follow-up plan with bounded vertical slices (if loading-shape
     fixes dominate), or
   - Both, if the gaps split cleanly.

Tasks 001–003 are independent and parallel-safe. The synthesis step is
sequential after all three land.

## Files To Inspect (shared context for tasks)

- `src/ui/shells/library/*.rs` — Library detail renderers.
- `src/view_models/library.rs` — Library projection types.
- `src/view_models/search_results/index_detail.rs` — Index detail VM.
- `src/view_models/search_results/mod.rs` — Index detail entry types.
- `src/ui/shells/search_results_inspector.rs` — Index detail render path.
- `src/discover/app_impl.rs` — parked but contains the historical
  field-surfacing reference for what Discover *used to* show; do not change.
- ADR 0024, ADR 0028, ADR 0029, ADR 0048, ADR 0049.

## Acceptance Criteria

- Three triage reports exist under `docs/reviews/`.
- Each report has the schema above filled per gap, with file:line evidence.
- Synthesis step produces one routing artifact (ADR draft or ADR 0024
  follow-up plan or both) that names every gap from the three reports and
  assigns it to a downstream packet.
- Deferred-architecture-work-index item #2 is moved to "Recently Resolved"
  once the synthesis artifact lands.

## Risk Areas

- Subagent confuses parked `src/discover/` rendering for the live Index
  detail path. Mitigation: the task files explicitly point at
  `src/view_models/search_results/index_detail.rs` as the live side and
  flag `src/discover/` as reference-only.
- Triage drifts into design proposals. Mitigation: report schema is
  observation-only; the "Routing" cell picks one of three named buckets
  and stops.

## References

- ADR 0052 — Library / Index data parity triage
- `docs/plans/deferred-architecture-work-index.md` (item #2)
- `docs/plans/post-adr-0028-follow-up-plan.md` (closed contributor parity)
- ADR 0024 — application layer query/service boundary
- ADR 0028 — local identity source-fact persistence
- ADR 0048 — ContentList frame breadcrumb search
- ADR 0049 — inspector source ownership
- `docs/notes/2026-05-discover-module-parked.md`
