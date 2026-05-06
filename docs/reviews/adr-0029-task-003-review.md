# ADR 0029 Task 003 Review

## Result

Pass - 2026-05-01.

## Scope

- `src/identity_ingest.rs`
- `src/search.rs`
- `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`

## Findings

- `identity_ingest::persist_musicindex_artist` is the single runtime place that
  maps `api::Artist` into `ArtistSourceFactInput`.
- Persistence requires a non-empty explicit `artist_id`; name-only artists are
  skipped.
- Replacement is still scoped to `(musicindex, artist_id)` and preserves other
  source rows with the same source artist id.
- Discover search result handling persists fetched `EntityDetail::Artist`
  records through the ingest helper.
- The synthetic Discover inspector artist path remains read-only and does not
  persist inferred names.
- No `tracks` artist-subject binding, person persistence, UI rendering change,
  or schema change was added.

## Verification

Green on 2026-05-01:

- `cargo test identity_ingest::tests::musicindex_artist`
- `cargo test search::tests::search_batch_persists_explicit_musicindex_artist_facts`
- `cargo fmt -- --check`
- `git diff --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

## Deferred

- Task 004 should hydrate local `ArtistView` for
  `ArtistRef::Musicindex(source_artist_id)` from `artist_source_facts`.
- Name-derived `ArtistRef::LocalArtistName` hydration remains unchanged until a
  future track-to-artist binding ADR.
- Artist source links/source ids remain empty until MusicIndex artist detail
  exposes those collections.
