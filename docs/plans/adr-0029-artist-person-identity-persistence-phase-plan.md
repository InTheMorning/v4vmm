# ADR 0029 Artist And Person Identity Persistence Phase Plan

## Status

Proposed - 2026-05-01.

## Goal

Persist artist/person identity facts locally without source inference, so
Library can eventually hydrate richer artist/person views from the same
provenance-first data used by Discover.

## Non-Goals

- Do not create a canonical global person registry in the first phase.
- Do not merge identities by display name.
- Do not change GPUI rendering before data contracts are clear.
- Do not change audio-tag writes.
- Do not add schema until Task 001 proves the source field shape.

## Assumptions

- ADR 0028 source-fact tables cover feed, track, and contributor owners, but not
  durable artist/person subjects.
- `ArtistView` can already represent identity links, ids, image, aliases, area,
  and active years.
- Local Library artist views are currently name-derived and can remain so until
  source-scoped facts are available.

## Affected Modules

- `docs/adr/0029-artist-person-identity-persistence.md`
- `src/views.rs`
- `src/sources.rs`
- `src/local_identity.rs`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/rss/subscribe.rs`
- `src/view_models/artist.rs`
- `tests/architecture_tests.rs`
- `migrations/`

## Proposed Sequence

1. Source inventory and schema decision.
   Task: `docs/tasks/adr-0029-task-001-source-inventory.md`.
2. Schema and DB helpers, only after Task 001 resolves the subject shape.
3. Ingest persistence for explicit MusicIndex/RSS artist/person facts.
4. Local view hydration for `ArtistView` and related Library projections.
5. Visual smoke and final architecture gates.

## Schema/API Implications

Expected but not approved yet. Task 001 must decide whether one
source-scoped subject fact family can cover both artists and people, or whether
artists and contributors/persons need separate table families.

Any eventual schema must define:

- owner/source identity keys
- replacement scope
- cascade behavior
- raw JSON retention
- conflict display rules
- migration tests

## Risk Areas

- Accidentally merging distinct artists because names match.
- Treating contributor position as durable person identity.
- Moving source-fact reconstruction into screens.
- Designing schema before the MusicIndex/RSS field inventory is complete.
- Overlapping with ADR 0028 feed/track/contributor facts instead of linking to
  them.

## Test Strategy

- Task 001 is documentation-only and needs no runtime tests unless code changes.
- Schema tasks must add migration and DB helper tests.
- Hydration tasks must add `ArtistView` and source/query tests.
- Architecture tests must prevent GPUI/screen imports in source-fact helpers.
- Full gate before any implementation commit:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Rollback Strategy

- Task 001 has no runtime rollback.
- Schema work must be additive.
- If ingest writes incorrect artist/person facts, disable the ingest call sites
  while leaving additive tables harmless.
- If hydration produces confusing display conflicts, keep persisted facts but
  hide the convenience display until the projection policy is revised.
