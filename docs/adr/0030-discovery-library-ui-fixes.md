# ADR 0030: Discovery and Library UI Correctness Fixes

## Status

Accepted - 2026-05-01.

## Context

Discovery and Library share feed, track, contributor, and metadata inspection
surfaces through ADR 0023, ADR 0025, ADR 0026, and ADR 0027. A reviewed plan in
`docs/plans/discovery-library-ui-fixes.md` identifies six visible correctness
issues in those surfaces:

- Discovery search forwards a backslash to the remote query parser.
- Discovery recent-feed tiles can lose visible title or artist labels.
- Feed headers arrange the same facts and actions differently across Library
  and Discovery.
- Discovery can expose Library-only compare actions.
- Contributor metadata cells can flatten person and role structure.
- Several detail panes do not scroll reliably in bounded flex layouts.

These are implementation correctness and presentation consistency fixes. They
do not require a new data model, persistence model, network source abstraction,
or screen ownership boundary.

## Decision

Execute the fixes as bounded task packets under the existing shared projection
and UI-composite architecture.

The implementation must extend existing helpers and contracts instead of
creating parallel ones:

- Query normalization remains in `src/api.rs`.
- Recents field population remains a deserialization/view fallback concern.
- Feed headers extend the existing `DetailHeader` and release detail slot
  pattern additively.
- Compare action visibility uses the existing `EntitySurfaceContext`.
- Contributor tree display reuses existing contributor grouping helpers.
- Scroll fixes use bounded flex children and preserve one vertical scroll view
  per detail surface.

## Alternatives Considered

- Create a new feed-header composite. Rejected because the existing
  `DetailHeader` and release detail slots already own this layout family.
- Add screen-local flags for compare visibility. Rejected because
  `EntitySurfaceContext` already expresses Discover versus Library.
- Rework Library and Discovery as a broader unification project. Rejected
  because the issues are smaller follow-ups and should not reopen ADR 0026.
- Normalize backslash only in the Discovery screen. Rejected because API query
  normalization already has one shared choke point.

## Consequences

- The changes stay small enough for one implementation packet at a time.
- UI fixes must not move service calls, DB reads, image-cache lookups, or click
  handlers into shared view models.
- The same command gates continue to apply: `cargo fmt -- --check`,
  `cargo check`, `cargo test`, and `cargo clippy -- -D warnings`.
- Manual visual smoke remains necessary for scroll and header layout behavior.

## Invariants

- `src/view_models/*` stays GPUI-free.
- Shared UI helpers stay free of screen modules and services.
- Metadata source facts are preserved; display formatting must not discard
  provenance.
- No schema migration is part of this ADR.
- No network API contract change is part of this ADR.
- Library-only actions must not appear in Discovery projections.

## Non-Goals

- No new persistence or migration work.
- No broad Discover/Library navigation redesign.
- No fuzzy metadata inference or source-fact merging.
- No replacement of ADR 0026 shared projection architecture.
- No change to download, subscription, playlist, playback, or MusicBrainz
  semantics except hiding irrelevant actions from Discovery.

## Follow-Up Work

Task packets:

- `docs/tasks/adr-0030-task-001-backslash-search.md`
- `docs/tasks/adr-0030-task-002-recents-labels.md`
- `docs/tasks/adr-0030-task-003-feed-header-parity.md`
- `docs/tasks/adr-0030-task-004-discovery-compare-actions.md`
- `docs/tasks/adr-0030-task-005-contributor-tree-metadata.md`
- `docs/tasks/adr-0030-task-006-scroll-containers.md`

Review checklist:

- `docs/reviews/adr-0030-review-checklist.md`
