# ADR 0029 Task 002: Artist Source Schema

## Status

Implemented - 2026-05-01.

## Goal

Add additive SQLite schema and DB helpers for explicit artist source facts keyed
by `(source, source_artist_id)`.

## Read

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-001-source-inventory.md`
- `docs/reviews/adr-0029-task-001-source-inventory-review.md`
- `src/db.rs`
- `src/views.rs`
- `src/local_identity.rs`
- `tests/architecture_tests.rs`
- existing `src/db.rs` migration registry

## Files Likely To Change

- `src/db.rs`
- `tests/architecture_tests.rs`
- `docs/tasks/adr-0029-task-002-artist-source-schema.md`
- `docs/reviews/adr-0029-task-002-review.md`

## Do Not Touch

- Do not change Library or Discover UI.
- Do not change MusicIndex or RSS ingest call sites.
- Do not hydrate `ArtistView` yet.
- Do not add global person tables.
- Do not merge identities by name, Nostr, role, or contributor position.

## Constraints

- Schema must be additive.
- Artist facts require non-empty `source` and non-empty `source_artist_id`.
- Local display names are not source artist ids.
- Artist source facts must not cascade from feed or track deletion.
- Raw source JSON must be preserved when available.
- Helper APIs must make replacement scope explicit: `(source, source_artist_id)`.
- Architecture tests must keep any new helper module UI-free.

## Implementation Steps

1. Add an additive `src/db.rs` registry migration for artist source facts.
2. Add DB input/row structs and replacement/read helpers.
3. Store typed artist display fields: name, sort name, image URL, website URL,
   aliases, tags, area, begin year, end year, observed time, raw JSON.
4. Store artist source links and source ids either in dedicated artist fact
   tables or in a constrained child-table shape tied to the artist source row.
5. Add migration/helper tests for insert, replace, read, and invalid empty keys.
6. Extend architecture tests if a new helper module is introduced.

## Acceptance Criteria

- [x] New schema is additive and migration-tested.
- [x] Replacement is source-scoped and does not rely on display name.
- [x] DB helpers can round-trip typed artist fields plus raw JSON.
- [x] Source links/ids can be stored without using feed/track owner tables.
- [x] No UI, MusicIndex ingest, RSS ingest, or `ArtistView` hydration behavior
  changes.
- [x] Required verification commands pass.

## Implementation Summary

- Added additive `artist_source_facts`, `artist_source_links`, and
  `artist_source_ids` schema creation under the existing `src/db.rs` migration
  registry.
- Added `ArtistSourceFactInput` / `ArtistSourceFactRow` and source-scoped
  replacement/read helpers keyed by `(source, source_artist_id)`.
- Stored artist display facts, aliases, tags, source links, source ids, and raw
  JSON without linking to local name-derived Library artists.
- Kept artist source facts independent of feed/track cascade behavior.
- Added focused DB tests for schema creation, round trip, replacement, and
  invalid keys.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test db::tests::test_artist_source
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

Verified 2026-05-01.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-002-artist-source-schema.md`
- `docs/reviews/adr-0029-task-001-source-inventory-review.md`
- `src/db.rs`
- `src/views.rs`
- `src/local_identity.rs`
- `tests/architecture_tests.rs`
- existing `src/db.rs` migration registry

Goal:
- Add additive SQLite schema and DB helpers for explicit artist source facts.

Constraints:
- No name-only identity matching.
- No global person tables.
- No UI changes.
- No ingest/hydration behavior changes.
- Replacement scope is `(source, source_artist_id)`.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- `src/rss/`
- `src/identity_ingest.rs`
- `src/sources.rs`
- MusicIndex client behavior

Acceptance criteria:
- Additive migration and DB helpers exist.
- Tests cover insert, replace, read, raw JSON retention, and invalid keys.
- Architecture boundaries remain green.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test db::tests::test_artist_source`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Schema requires linking source artists to local name-derived Library artists.
- MusicIndex artist detail lacks stable `artist_id`.
- The implementation needs to hydrate `ArtistView` to prove the schema.
