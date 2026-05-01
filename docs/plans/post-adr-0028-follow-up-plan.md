# Post-ADR 0028 Follow-Up Plan

## Status

Implemented - 2026-05-01.

## Goal

Close the bounded visibility gap left after ADR 0028 by making already-hydrated
local contributor identity facts visible in Library release detail without
changing source-fact persistence, ingest, schema, or artist/person
reconciliation.

## Non-Goals

- Do not add a global artist, contributor, or person registry.
- Do not infer contributors from names, titles, tags, publisher text, or
  filenames.
- Do not add another SQLite migration.
- Do not change Discover lazy contributor fetching semantics.
- Do not redesign Library or Discover release detail.

## Assumptions

- ADR 0028 already persists and hydrates feed contributor facts into
  `FeedView::contributors`.
- Library album detail already renders from a hydrated `FeedView`.
- Contributor identity click behavior remains screen-owned.
- Shared GPUI layout helpers may live under `src/ui_entity.rs` if they stay
  independent of screen modules, services, DB, and API row types.

## Affected Modules

- `src/ui_entity.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. Library contributor panel slot. Completed 2026-05-01.
   Task: `docs/tasks/post-adr-0028-task-001-library-contributor-panel.md`.
2. Future artist/person identity ADR only if product behavior requires
   cross-feed reconciliation.

## Risk Areas

- Accidentally adding a lazy network fetch path to Library instead of rendering
  already-hydrated facts.
- Moving screen-owned click behavior into shared layout helpers.
- Reintroducing `api::Contributor` or screen-local contributor projections.
- Expanding into artist/person identity matching before rules exist.

## Test Strategy

- Unit or source-scan coverage that contributor rendering stays projection-led.
- Existing view-model tests for contributor grouping and local hydration.
- Architecture tests for shared projection boundaries.
- Full Rust gate before merge.

## Rollback Strategy

- The panel slot is additive. If it causes visual or runtime issues, remove the
  Library `after_section` contributor panel call while keeping ADR 0028
  persistence intact.

## Deferred Work

- A future ADR may introduce durable artist/person identity persistence,
  matching rules, and conflict policy.
- Broader Library/Discover visual parity remains tracked outside ADR 0028; this
  plan only closed the contributor identity visibility gap.
