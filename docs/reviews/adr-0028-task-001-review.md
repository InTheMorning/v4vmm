# ADR 0028 Task 001 Review

## Result

Pass - 2026-05-01.

## Reviewed Scope

- `src/db.rs`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`

## Findings

No blocking findings.

## Architecture Check

- The change is additive schema plus DB helpers only.
- No MusicIndex, RSS, subscription, Library, Discover, GPUI, or projection
  hydration call sites were wired.
- Helper APIs use DB-owned local owner/input/row types, not concrete API rows.
- Source-scoped replacement is transactional and preserves unrelated sources.
- Discriminator `CHECK` constraints enforce owner-column shapes at SQLite level.
- Feed and track foreign keys use `ON DELETE CASCADE`.

## Tests

Green:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test db::tests`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

## Merge Recommendation

Task 001 can merge. Task 002 should wire MusicIndex/RSS ingest to these helpers
without changing projection behavior.
