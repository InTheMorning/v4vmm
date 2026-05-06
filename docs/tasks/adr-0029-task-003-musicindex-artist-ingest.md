# ADR 0029 Task 003: MusicIndex Artist Ingest

## Status

Implemented - 2026-05-01.

## Goal

Persist explicit MusicIndex artist detail records into the ADR 0029 artist
source-fact tables without inferring identity from names or local tracks.

## Read

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-002-artist-source-schema.md`
- `docs/reviews/adr-0029-task-002-review.md`
- `src/api.rs`
- `src/identity_ingest.rs`
- `src/search.rs`
- `src/sources.rs`
- `src/db.rs`

## Files Likely To Change

- `src/identity_ingest.rs`
- `src/search.rs`
- `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`
- `docs/reviews/adr-0029-task-003-review.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`

## Do Not Touch

- Do not change `api::Client` fetch semantics.
- Do not persist synthetic Discover inspector artists built from track search.
- Do not change `ArtistView` hydration yet.
- Do not change `tracks` schema or bind tracks to artist subjects.
- Do not introduce global person tables or person merge behavior.

## Constraints

- Persist only when `api::Artist.artist_id` is present and non-empty.
- Use `(source = "musicindex", source_artist_id = artist_id)` as the key.
- Do not derive source artist ids from display names.
- Do not invent artist source links or source ids; persist empty vectors until
  the MusicIndex artist API exposes those fields.
- Preserve raw artist JSON when serialization succeeds.
- Keep artist fact construction inside `identity_ingest.rs`.

## Implementation Steps

1. Add `identity_ingest::persist_musicindex_artist`.
2. Map `api::Artist` scalar fields into `db::ArtistSourceFactInput`.
3. Add tests for persist, skip missing id, replace same MusicIndex key, and
   preserve other source rows.
4. Call the helper from the search result path only for fetched
   `EntityDetail::Artist` records that carry explicit MusicIndex ids.
5. Update this task and add a task review after verification.

## Acceptance Criteria

- [x] Explicit MusicIndex artist detail rows are persisted locally.
- [x] Artists without explicit `artist_id` are skipped.
- [x] Replacement remains scoped to `(musicindex, artist_id)`.
- [x] Other source rows with the same source artist id survive.
- [x] No synthetic artist names are persisted.
- [x] Required verification commands pass.

## Implementation Summary

- Added `identity_ingest::persist_musicindex_artist`, which maps explicit
  `api::Artist.artist_id` records into `db::ArtistSourceFactInput`.
- Persisted scalar fields, aliases, tags, active years, `updated_at` as
  `observed_at`, and raw artist JSON.
- Kept source links and source ids empty because `api::Artist` does not expose
  artist source-fact collections yet.
- Wired Discover search results to persist fetched `EntityDetail::Artist` rows
  through the ingest helper.
- Kept synthetic Discover inspector artists and name-only artist rows out of
  persistence.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test identity_ingest::tests::musicindex_artist
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

Verified 2026-05-01.

Additional focused check:

```bash
cargo test search::tests::search_batch_persists_explicit_musicindex_artist_facts
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`
- `src/api.rs`
- `src/identity_ingest.rs`
- `src/search.rs`
- `src/db.rs`

Goal:
- Persist explicit MusicIndex artist detail records into artist source facts.

Constraints:
- No name inference.
- No synthetic inspector artist persistence.
- No `tracks` artist binding.
- No `ArtistView` hydration changes.
- Keep fact construction in `identity_ingest.rs`.

Do not touch:
- `src/api.rs` fetch semantics
- `src/views.rs`
- `src/sources.rs`
- `src/rss/`
- schema migrations

Acceptance criteria:
- Explicit `artist_id` records persist under `(musicindex, artist_id)`.
- Missing/empty artist ids are skipped.
- Same-key MusicIndex rows replace.
- Other source rows remain.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test identity_ingest::tests::musicindex_artist`
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

- MusicIndex artist detail does not include `artist_id`.
- Persistence requires changing `api::Client` or endpoint contracts.
- The implementation needs name-based matching to find a local artist.
