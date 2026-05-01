# ADR 0028 Local Identity Source Fact Persistence Phase Plan

## Status

Proposed - 2026-05-01.

## Goal

Persist MusicIndex and RSS identity source facts locally so Library can hydrate
the same GPUI-free identity projections as Discover when facts are known.

## Non-Goals

- Do not build a global artist or contributor registry.
- Do not infer identity from names, titles, publisher text, filenames, or fuzzy
  matching.
- Do not move database access into `src/views.rs`,
  `src/view_models/entity_detail.rs`, or `src/ui_entity.rs`.
- Do not change artwork resolver ownership.
- Do not redesign Library or Discover visuals.

## Current State

- ADR 0026 projections can represent `source_links`, `source_ids`, and
  contributor `href` / `img` / `npub`.
- Discover can preserve those facts while API rows are loaded in memory.
- Local Library rows do not have normalized source-fact storage for those facts.
- Existing local JSON columns retain some RSS-specific data but are not a
  shared source-fact read model.

## Target State

- SQLite stores identity links, identity ids, and contributor identity rows
  under local feed/track owners.
- MusicIndex and RSS ingest workflows persist known source facts without
  inference.
- Local feed and track queries hydrate `FeedView`, `TrackView`, and
  `ContributorView` with persisted source facts.
- Library and Discover identity affordances differ only when a source fact is
  genuinely unavailable.

## Affected Modules

- `src/db.rs`
- `src/rss/subscribe.rs`
- `src/rss/enrich.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `src/views.rs`
- `src/library_service.rs`
- `src/application/queries/library.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. Schema and local read/write helpers.
   Task: `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`.
2. Ingest persistence for MusicIndex and RSS source facts.
   Task: `docs/tasks/adr-0028-task-002-ingest-persistence.md`.
3. Local view hydration for feed, track, and contributor facts.
   Task: `docs/tasks/adr-0028-task-003-local-view-hydration.md`.
4. Library/Discover identity visual smoke.
   Task: `docs/tasks/adr-0028-task-004-identity-visual-smoke.md`.
5. Cleanup and architecture gates.
   Task to create only if phases 1-4 add compatibility shims.

## Task 001 Scope

- Add the source-fact tables and indexes.
- Add DB row structs and helper functions for replacing and loading facts by
  local owner.
- Add discriminator `CHECK` constraints, source-scoped replacement helpers,
  and tests for round-trip preservation, source separation, invalid owner
  shapes, and cascade/delete behavior.
- Do not wire ingest or UI hydration yet.

## Schema/API Implications

- SQLite schema changes are required.
- No public MusicIndex API change is required.
- Existing API structs remain source inputs; shared view facts continue using
  local `views::*` fact types.

## Risk Areas

- Accidentally deleting facts from unrelated sources during refresh.
- Treating a convenience identity field as canonical and losing raw facts.
- Letting screen code parse source-fact JSON directly.
- Making contributor positions appear more stable than they are.
- Expanding into global artist/person reconciliation before local persistence
  works.
- Adding contributor scalar columns without preserving matching raw facts when
  the source provides generic contributor link/id rows.

## Test Strategy

- DB unit tests for schema migration and helper round trips.
- Tests that source-specific replacement leaves unrelated sources intact.
- Tests that invalid discriminator/owner column combinations are rejected.
- View tests for hydrating `EntityIdentityLinks` from persisted local facts.
- Architecture tests that keep shared projections database-free.
- Manual visual smoke after Library hydration.

Required verification before each implementation commit:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking a task implemented.

## Rollback Strategy

- Task 001 should be additive schema and helper code only.
- If ingest persistence causes incorrect facts, disable the ingest call sites
  while leaving the additive schema harmless.
- Do not remove existing JSON fields or scalar metadata during this ADR.
