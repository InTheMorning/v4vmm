# ADR 0029 Artist Identity Persistence Phase Plan

## Status

In progress - 2026-05-01. Tasks 001-004 implemented.

## Goal

Persist explicit artist identity facts locally without source inference, so
Library can eventually hydrate richer artist views from the same
provenance-first data used by Discover. Person identity remains deferred.

## Non-Goals

- Do not create a canonical global artist or person registry in this ADR.
- Do not persist global person identity in this ADR.
- Do not merge identities by display name.
- Do not change GPUI rendering before data contracts are clear.
- Do not change audio-tag writes.
- Do not bind local `tracks` rows to artist subjects in this ADR.

## Assumptions

- ADR 0028 source-fact tables cover feed, track, and contributor owners, but not
  durable artist subjects.
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
- existing `src/db.rs` migration registry

## Proposed Sequence

1. Source inventory and schema decision. Applied 2026-05-01.
   Task: `docs/tasks/adr-0029-task-001-source-inventory.md`.
2. Artist source-fact schema and DB helpers for explicit artist subjects.
   Implemented 2026-05-01.
   Task: `docs/tasks/adr-0029-task-002-artist-source-schema.md`.
3. Ingest persistence for explicit MusicIndex artist records.
   Implemented 2026-05-01.
   Task: `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`.
4. Local explicit-source artist lookup/hydration for `ArtistView`.
   Implemented 2026-05-01.
   Task: `docs/tasks/adr-0029-task-004-local-artist-source-hydration.md`.
5. Visual smoke and final architecture gates.
   Next task: `docs/tasks/adr-0029-task-005-final-gates.md`.

## Schema/API Implications

Expected for explicit artist subjects. Task 001 recommends split schema tracks:
artist source-fact storage first, with contributor/person facts remaining
owner-scoped under ADR 0028 until a source provides explicit durable person ids
and a later ADR defines merge policy.

Any eventual schema must define:

- owner/source identity keys
- replacement scope
- lifecycle and non-cascade behavior
- raw JSON retention
- conflict display rules
- migration tests

## Risk Areas

- Accidentally merging distinct artists because names match.
- Treating contributor position as durable person identity.
- Pretending local name-derived artists have explicit source ids.
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
- If ingest writes incorrect artist facts, disable the ingest call sites while
  leaving additive tables harmless.
- If hydration produces confusing display conflicts, keep persisted facts but
  hide the convenience display until the projection policy is revised.
