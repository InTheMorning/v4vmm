# ADR 0029 Task 002 Review

## Result

Pass - 2026-05-01.

## Scope

- `src/db.rs`
- `docs/tasks/adr-0029-task-002-artist-source-schema.md`

## Findings

- Schema changes are additive and live in the existing DB migration registry.
- Artist facts are keyed by explicit `(source, source_artist_id)` values, not
  display names.
- Source links and ids are stored in artist-specific child tables, separate
  from ADR 0028 feed/track owner tables.
- Artist source facts do not cascade from feed or track deletion, matching the
  ADR lifecycle rule.
- No local track-to-artist-subject binding was added.
- No UI, MusicIndex ingest, RSS ingest, or `ArtistView` hydration behavior was
  changed.

## Verification

Green on 2026-05-01:

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo check`
- `cargo test db::tests::test_artist_source`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

## Deferred

- Task 003 should add ingest persistence for explicit MusicIndex artist records.
- `ArtistView` hydration remains deferred until source facts are written by an
  ingest/query path.
- Local name-derived artist hydration remains deferred until a future ADR
  defines track-to-artist-subject binding.
- Global person identity remains deferred until durable person ids and merge
  policy exist.
