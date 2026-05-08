# ADR 0045 Task 002: MusicIndex Binding Ingest

## Goal

Persist track-to-artist bindings when MusicIndex responses provide explicit
artist ids already accepted by ADR 0029.

Status: Implemented - 2026-05-08.

## Files to Inspect

- `docs/adr/0045-track-artist-binding.md`
- `docs/tasks/adr-0045-task-001-track-artist-binding-schema.md`
- `src/api.rs`
- `src/identity_ingest.rs`
- `src/search/app_impl.rs`
- `src/search/tests.rs`

## Files Likely to Change

- `src/identity_ingest.rs`
- `src/search/app_impl.rs` only at existing ingest call sites
- focused tests near existing MusicIndex artist persistence tests

## Do Not Touch

- `src/ui/**`
- `src/views.rs`
- `src/view_models/**`
- Audio tag write paths

## Constraints

- Persist only explicit artist ids.
- Name-only artists must not create bindings.
- Existing artist source-fact replacement semantics remain unchanged.
- Binding write failures must surface through existing command/error paths.

## Implementation Steps

1. Done: locate where MusicIndex artist facts are persisted for tracks.
2. Done: add binding inputs only when local track id and explicit artist id are
   both present.
3. Done: preserve role/provenance fields so the read model can explain the
   binding.
4. Done: add tests proving explicit ids bind and name-only responses do not.

## Acceptance Criteria

- [x] Explicit MusicIndex artist ids create track bindings.
- [x] Name-only artists do not create bindings.
- [x] Existing ADR 0029 artist source-fact tests still pass.

## Implementation Notes

- `api::ArtistCredit` now accepts explicit `artist_id` from MusicIndex track
  payloads.
- `identity_ingest::persist_musicindex_track()` writes source-scoped
  MusicIndex bindings only from non-empty `artist_credit.artist_id` values.
- Ingest creates a minimal MusicIndex artist source fact only when the explicit
  artist id is missing from local storage; existing richer artist facts are not
  replaced.
- Name-only or missing artist credits clear MusicIndex-owned bindings for the
  track without creating new artist source facts.
- No UI, view-model, Library hydration, or audio-tag paths changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test musicindex_artist_source_fact
cargo test track_artist_source_binding
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0045-track-artist-binding.md`
- `src/identity_ingest.rs`
- `src/search/app_impl.rs`
- `src/search/tests.rs`

Goal:
- Persist track-to-artist bindings from explicit MusicIndex artist ids.

Constraints:
- No name matching.
- No UI changes.
- No view-model hydration changes.

Do not touch:
- `src/ui/**`
- `src/views.rs`
- `src/view_models/**`
- Audio tag write paths

Acceptance criteria:
- Explicit ids bind; name-only artists do not.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test musicindex_artist_source_fact`
- `cargo test track_artist_source_binding`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- MusicIndex responses do not expose local track ids at the ingest point.
- Binding requires a new public API payload.
