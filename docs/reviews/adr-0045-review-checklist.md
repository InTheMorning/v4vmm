# ADR 0045 Review Checklist

## Reviewed Artifacts

- `docs/adr/0045-track-artist-binding.md`
- `docs/plans/adr-0045-track-artist-binding-phase-plan.md`
- `docs/tasks/adr-0045-task-001-track-artist-binding-schema.md`
- `docs/tasks/adr-0045-task-002-musicindex-binding-ingest.md`
- `docs/tasks/adr-0045-task-003-library-artist-hydration.md`
- `docs/tasks/adr-0045-task-004-guards-and-readiness.md`

## Gate Status

Status: Task 001 implemented on 2026-05-08. Tasks 002-004 pending.

## Required Checks

- [x] Binding schema is additive.
- [x] Bindings require explicit `(source, source_artist_id)`.
- [x] Track removal deletes bindings but not artist source facts.
- [ ] MusicIndex ingest writes bindings only from explicit artist ids.
- [ ] Name-only artists do not create bindings.
- [ ] Library artist hydration uses read-model helpers, not renderer inference.
- [ ] Multiple bound subjects under one local display name are not silently
      merged.
- [ ] Architecture tests block screen-side binding inference.
- [x] `cargo fmt -- --check` green.
- [x] `cargo check` green.
- [x] `cargo test` green.
- [x] `cargo clippy -- -D warnings` green.

## Required Fixes

- Tasks 002-004 remain pending.

## Optional Improvements

- Consider a later visual smoke if enriched artist identity changes visible
  Library artist header density.

## Architectural Drift

- No drift in Task 001. Schema and helpers stayed in `src/db.rs`;
  no UI, ingest, read-model, or audio tag behavior changed.

## Missing Tests

- Full ADR 0045 still needs ingest, hydration, guard, and full-suite
  readiness tests.

## Merge Recommendation

Do not merge runtime changes until Tasks 001-004 pass this checklist.
