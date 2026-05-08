# ADR 0045 Task 001: Track Artist Binding Schema

## Goal

Add additive SQLite schema and DB helpers for explicit track-to-artist source
bindings.

## Files to Inspect

- `docs/adr/0045-track-artist-binding.md`
- `docs/plans/adr-0045-track-artist-binding-phase-plan.md`
- `src/db.rs`
- `src/views.rs`

## Files Likely to Change

- `src/db.rs`
- `tests/architecture_tests.rs` only if a new guard is needed for the schema
  boundary

## Do Not Touch

- `src/library/app_impl.rs`
- `src/search/app_impl.rs`
- `src/ui/**`
- Audio tag write paths

## Constraints

- Schema must be additive.
- Binding requires non-empty `(source, source_artist_id)`.
- Bindings must not create or delete artist source facts.
- Use explicit helper names; do not hide replacement scope.

## Implementation Steps

1. Add a migration in `src/db.rs` for `track_artist_source_bindings`.
2. Include indexes for `track_id`, `(source, source_artist_id)`, and role.
3. Add input/row structs for binding replacement and query helpers.
4. Add helper tests for insert, replacement, required keys, and track-delete
   cascade behavior.

## Acceptance Criteria

- Fresh and migrated databases include the binding table.
- Bindings cannot be inserted with blank source or source artist id.
- Removing a track removes its bindings but not artist source facts.
- No UI or ingest behavior changes.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test db::tests::test_track_artist_source_binding
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0045-track-artist-binding.md`
- `docs/plans/adr-0045-track-artist-binding-phase-plan.md`
- `src/db.rs`

Goal:
- Add additive SQLite schema and DB helpers for explicit track-to-artist source
  bindings.

Constraints:
- No name matching.
- No UI changes.
- No artist source fact deletion.

Do not touch:
- `src/library/app_impl.rs`
- `src/search/app_impl.rs`
- `src/ui/**`
- Audio tag write paths

Acceptance criteria:
- DB tests prove insert, replacement, required keys, and track-delete cascade.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test db::tests::test_track_artist_source_binding`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Existing schema cannot express track cascade without broad migration changes.
- A helper needs to infer artist subjects from display names.
