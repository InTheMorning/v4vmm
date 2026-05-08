# ADR 0045 Review Checklist

## Reviewed Artifacts

- `docs/adr/0045-track-artist-binding.md`
- `docs/plans/adr-0045-track-artist-binding-phase-plan.md`
- `docs/tasks/adr-0045-task-001-track-artist-binding-schema.md`
- `docs/tasks/adr-0045-task-002-musicindex-binding-ingest.md`
- `docs/tasks/adr-0045-task-003-library-artist-hydration.md`
- `docs/tasks/adr-0045-task-004-guards-and-readiness.md`

## Gate Status

Status: Not started.

## Required Checks

- [ ] Binding schema is additive.
- [ ] Bindings require explicit `(source, source_artist_id)`.
- [ ] Track removal deletes bindings but not artist source facts.
- [ ] MusicIndex ingest writes bindings only from explicit artist ids.
- [ ] Name-only artists do not create bindings.
- [ ] Library artist hydration uses read-model helpers, not renderer inference.
- [ ] Multiple bound subjects under one local display name are not silently
      merged.
- [ ] Architecture tests block screen-side binding inference.
- [ ] `cargo fmt -- --check` green.
- [ ] `cargo check` green.
- [ ] `cargo test` green.
- [ ] `cargo clippy -- -D warnings` green.

## Required Fixes

- Pending implementation.

## Optional Improvements

- Consider a later visual smoke if enriched artist identity changes visible
  Library artist header density.

## Architectural Drift

- Pending implementation review.

## Missing Tests

- Pending implementation.

## Merge Recommendation

Do not merge runtime changes until Tasks 001-004 pass this checklist.
