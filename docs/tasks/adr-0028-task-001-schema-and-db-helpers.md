# ADR 0028 Task 001: Schema And DB Helpers

## Status

Proposed.

## Goal

Add additive SQLite storage and focused DB helpers for local identity source
facts without wiring ingest workflows or UI hydration yet.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/reviews/post-adr-0026-task-002-identity-persistence-audit.md`
- `src/db.rs`
- `src/views.rs`
- `src/rss/subscribe.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`

## Files Likely To Change

- `src/db.rs`
- `tests/architecture_tests.rs`, only if a new boundary check is needed
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `docs/reviews/adr-0028-task-001-review.md`

## Do Not Touch

- Do not wire MusicIndex, RSS, subscription, feed update, Library, or Discover
  call sites in this task.
- Do not change shared projection behavior.
- Do not add GPUI imports to `src/views.rs` or `src/view_models/*`.
- Do not remove or repurpose existing JSON columns.
- Do not introduce global artist/person identity matching.

## Constraints

- Schema must be additive.
- Preserve raw facts and provenance; do not store only convenience fields.
- Replacement helpers must be source-scoped so one refresh cannot delete facts
  from unrelated sources.
- Discriminator tables must use SQLite `CHECK` constraints to enforce valid
  `owner_kind` and owner-column combinations.
- Source-scoped replacement helpers must require an explicit source token.
- Feed-owned facts and track-owned facts must use `ON DELETE CASCADE`.
- Helper APIs should use local view fact structs or DB-owned row structs, not
  concrete `api::*` source rows.
- `raw_json` should be written whenever the original source row is available
  and should not be parsed by current display read models.
- Follow existing `db.rs` error and test style.

## Implementation Steps

1. Add tables for identity links, identity ids, and contributor identity rows
   to `db::init_schema`.
2. Add indexes for owner lookup and foreign-key cleanup.
3. Add `CHECK` constraints for each valid `owner_kind` shape.
4. Add DB-owned row structs or helper input structs for:
   - local identity links
   - local identity ids
   - local contributors
5. Add helper functions to replace facts by owner and explicit source in one
   transaction.
6. Add helper functions to load facts by feed and by track.
7. Add focused DB tests:
   - fresh schema creates the new tables
   - link/id/contributor facts round-trip without losing provenance
   - replacing facts for one source leaves another source intact
   - invalid owner/discriminator combinations are rejected
   - deleting a feed deletes feed and track-owned facts
8. Update this task and add a review file with verification results.

## Acceptance Criteria

- [ ] Schema is additive and existing tests still pass.
- [ ] DB helpers can round-trip source links, source ids, and contributors.
- [ ] Source-scoped replacement behavior is covered by tests.
- [ ] Invalid owner/discriminator combinations are rejected by tests.
- [ ] Delete/cascade behavior is covered by tests.
- [ ] No ingest or UI call sites are wired in this task.
- [ ] Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test db::tests
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Expected Final Report

1. Files changed.
2. Tests run.
3. Schema additions.
4. Behavior intentionally not wired yet.
5. Unresolved concerns for Task 002.

## Escalation Triggers

- The existing schema migration discipline cannot support additive tables in
  `init_schema`.
- Source-scoped replacement cannot be made transactional without changing
  ingest workflow ownership.
- Helper APIs would need to expose concrete MusicIndex API structs from the DB
  boundary.
