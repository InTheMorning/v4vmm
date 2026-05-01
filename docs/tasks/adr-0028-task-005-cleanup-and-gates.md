# ADR 0028 Task 005: Cleanup and Gates

## Status

Implemented.

## Goal

Finalize ADR 0028 by removing the compatibility duplication added during visual
smoke, tightening architecture-test coverage, and marking the ADR implemented
after verification gates pass.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0028-task-002-ingest-persistence.md`
- `docs/tasks/adr-0028-task-003-local-view-hydration.md`
- `docs/tasks/adr-0028-task-004-identity-visual-smoke.md`
- `src/local_identity.rs`
- `src/sources.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/local_identity.rs`
- `src/lib.rs`
- `src/sources.rs`
- `src/library.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0028-task-005-cleanup-and-gates.md`
- `docs/reviews/adr-0028-task-005-review.md`

## Do Not Touch

- Do not add another schema migration.
- Do not change MusicIndex, RSS, subscription, download, playlist, playback, or
  MusicBrainz behavior.
- Do not redesign Library or Discover visuals.
- Do not move SQLite access into `src/views.rs` or
  `src/view_models/entity_detail.rs`.

## Constraints

- Keep source-fact mapping GPUI-free and reusable by local query/screen
  adapters.
- Keep architecture tests source-scan based, matching ADR 0023/0025 precedent.
- Mark ADR 0028 implemented only after all task gates are represented in code,
  docs, and verification.
- Keep deferred artist/person identity work explicit.

## Implementation Summary

- Extracted local SQLite source-fact row mapping into `src/local_identity.rs`.
- Updated `LocalSource` and Library album snapshot construction to use the same
  mapper.
- Added `src/local_identity.rs` to the non-UI core architecture-test boundary.
- Marked ADR 0028 and its phase plan implemented.
- Documented remaining deferred work as future artist/person identity
  reconciliation.

## Acceptance Criteria

- [x] `src/sources.rs` and `src/library.rs` no longer duplicate source-fact row
  mapping.
- [x] `src/local_identity.rs` stays GPUI-free under architecture tests.
- [x] ADR 0028 status reflects the implemented state.
- [x] Remaining deferred work is explicit and does not weaken ADR 0028 done
  criteria.
- [x] Verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test sources::tests::local_source_fetch_feed_hydrates_feed_and_track_identity_facts
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Escalation Triggers

- Cleanup requires changing schema or ingest behavior.
- Architecture gates reveal broader GPUI/service coupling outside ADR 0028.
- Verification fails outside the files touched by this task.
